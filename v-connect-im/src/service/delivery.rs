use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::domain::message::{HttpSendMessageRequest, HttpSendMessageResponse, ImMessage};
use crate::server::VConnectIMServer;
use crate::storage;

impl VConnectIMServer {
    /// 通过 HTTP 接口发送单聊消息 / Send a direct message through the HTTP API.
    ///
    /// # 参数 Parameters
    /// * `request` - HTTP 请求体，包含发送方、接收方及消息内容 / The HTTP payload describing sender, recipient, and payload.
    ///
    /// # 返回 Returns
    /// * `HttpSendMessageResponse` - 返回发送结果、消息 ID 及送达时间 / Result describing success flag, message id, and delivery timestamp.
    pub async fn http_send_message(
        &self,
        request: HttpSendMessageRequest,
    ) -> HttpSendMessageResponse {
        let message_id = Uuid::new_v4().to_string();
        let delivered_at = chrono::Utc::now().timestamp_millis();
        let message_type = request
            .message_type
            .clone()
            .unwrap_or_else(|| "message".to_string());

        // 调用插件系统处理消息 / Call plugin system to process message
        if let Some(pool) = self.plugin_connection_pool.as_ref() {
            tracing::info!(
                "🔌 调用插件系统处理消息 / Calling plugin system for message: {}",
                message_id
            );
            let plugin_message = serde_json::json!({
                "message_id": message_id,
                "from_uid": request.from_uid,
                "to_uid": request.to_uid,
                "content": request.content,
                "message_type": message_type,
                "timestamp": delivered_at
            });

            match pool.broadcast_message_event(&plugin_message).await {
                Ok(responses) => {
                    tracing::info!(
                        "✅ 插件处理响应数量 / Plugin response count: {}",
                        responses.len()
                    );
                    tracing::debug!("插件处理响应详情 / Plugin responses: {:?}", responses);
                    // 检查是否有插件要求停止消息传播 / Check if any plugin wants to stop propagation
                    for (plugin_name, response) in responses {
                        tracing::debug!(
                            "插件 {} 响应 / Plugin {} response: {}",
                            plugin_name,
                            plugin_name,
                            response
                        );
                        if let Some(flow) = response.get("flow").and_then(|v| v.as_str()) {
                            if flow == "stop" {
                                tracing::info!(
                                    "🛑 消息被插件 {} 拦截 / Message stopped by plugin {}",
                                    plugin_name,
                                    plugin_name
                                );
                                return HttpSendMessageResponse {
                                    success: false,
                                    message: format!("Message blocked by plugin {}", plugin_name),
                                    message_id: Some(message_id),
                                    delivered_at: Some(delivered_at),
                                };
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ 插件系统调用失败 / Plugin system call failed: {}", e);
                }
            }
        } else {
            tracing::warn!("⚠️  插件连接池未初始化 / Plugin connection pool not initialized");
        }

        let forward_msg = ImMessage {
            msg_type: "forwarded_message".to_string(),
            data: serde_json::json!({
                "from": request.from_uid,
                "content": request.content,
                "timestamp": delivered_at,
                "message_id": message_id
            }),
            target_uid: None,
        };
        let forward_json = serde_json::to_string(&forward_msg).unwrap_or_default();

        // 保存消息到存储插件 / Save message to storage plugin
        if let Some(pool) = self.plugin_connection_pool.as_ref() {
            match pool
                .storage_save_message(
                    &message_id,
                    &request.from_uid,
                    &request.to_uid,
                    &request.content,
                    delivered_at,
                    &message_type,
                    None,
                )
                .await
            {
                Ok(true) => {
                    tracing::debug!("💾 消息已保存到存储插件 / Message saved to storage plugin");
                }
                Ok(false) => {
                    tracing::warn!("⚠️  存储插件保存失败 / Storage plugin save failed");
                }
                Err(e) => {
                    tracing::error!("❌ 存储插件错误 / Storage plugin error: {}", e);
                }
            }
        } else {
            tracing::warn!("⚠️  插件连接池未初始化，消息未保存 / Plugin pool not initialized, message not saved");
        }

        // 保留 Raft 日志（用于集群同步）/ Keep Raft log (for cluster sync)
        let record = storage::MessageRecord {
            message_id: message_id.clone(),
            from_client_id: request.from_uid.clone(),
            to_client_id: request.to_uid.clone(),
            content: request.content.clone(),
            timestamp: delivered_at,
            msg_type: message_type.clone(),
            room_id: None,
        };
        let _ = self.raft.append_entry_as(&self.node_id, &record);

        let mut in_memory_delivery = false;
        if let Some(clients) = self.uid_clients.get(&request.to_uid) {
            for cid in clients.iter() {
                if self
                    .send_message_to_client(&cid, Message::Text(forward_json.clone()))
                    .await
                    .is_ok()
                {
                    in_memory_delivery = true;
                }
            }
        }

        if !in_memory_delivery {
            let ack_deadline = v::get_global_config_manager()
                .ok()
                .map(|cm| cm.get_or("delivery.ack_deadline_ms", 1000_u64))
                .unwrap_or(1000);
            self.await_ack_or_queue_offline(
                request.to_uid.clone(),
                message_id.clone(),
                None,
                request.content.clone(),
                message_type.clone(),
                ack_deadline,
            )
            .await;
        }

        HttpSendMessageResponse {
            success: true,
            message: "ok".to_string(),
            message_id: Some(message_id),
            delivered_at: Some(delivered_at),
        }
    }

    /// 等待 ACK 或写入离线消息 / Await ACK for a message or enqueue it as offline storage.
    ///
    /// # 参数 Parameters
    /// * `recipient_uid` - 接收方 UID / Recipient UID.
    /// * `message_id` - 消息唯一标识 / Unique message identifier.
    /// * `room_id` - 可选的房间 ID / Optional room id when message targets a room.
    /// * `content` - 消息内容 JSON / Message payload content.
    /// * `msg_type` - 消息类型字符串 / Message type label.
    /// * `deadline_ms` - 等待 ACK 的毫秒数 / Deadline (ms) to wait for ACK before queuing offline.
    ///
    /// # 返回 Returns
    /// * `()` - 异步任务内部处理结果，无显式返回 / No direct return value; the spawned task handles persistence.
    pub async fn await_ack_or_queue_offline(
        &self,
        recipient_uid: String,
        message_id: String,
        room_id: Option<String>,
        content: serde_json::Value,
        msg_type: String,
        deadline_ms: u64,
    ) {
        let server = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(deadline_ms)).await;
            let acked = server
                .acked_ids
                .get(&recipient_uid)
                .map(|set| set.contains(&message_id))
                .unwrap_or(false);
            if acked {
                return;
            }

            let _ = server.enforce_offline_quota_for_uid(&recipient_uid).await;

            // 保存离线消息到存储插件 / Save offline message to storage plugin
            let timestamp = chrono::Utc::now().timestamp_millis();
            if let Some(pool) = server.plugin_connection_pool.as_ref() {
                match pool
                    .storage_save_offline(
                        &message_id,
                        None,
                        &recipient_uid,
                        &content,
                        timestamp,
                        &msg_type,
                        room_id.as_deref(),
                    )
                    .await
                {
                    Ok(true) => {
                        tracing::debug!(
                            "💾 离线消息已保存 / Offline message saved: {}",
                            message_id
                        );
                    }
                    Ok(false) => {
                        tracing::warn!("⚠️  离线消息保存失败 / Offline message save failed");
                    }
                    Err(e) => {
                        tracing::error!("❌ 离线消息保存错误 / Offline message save error: {}", e);
                    }
                }
            }
            // server  // 已移除 / Removed
            //     .send_message_webhook(
            //         &message_id,
            //         &recipient_uid,
            //         &None,
            //         &None,
            //         &Some(recipient_uid.clone()),
            //         &content,
            //         &msg_type,
            //         "queued_offline",
            //         None,
            //     )
            //     .await;
        });
    }
}
