use crate::plugins::{PluginContext, PluginFlow};
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::Parser;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use v::init_tracing;

include!(concat!(env!("OUT_DIR"), "/auto_mod.rs"));
// 为生成器提供 crate 别名（2018 edition 兼容写法）
extern crate self as v_connect_im;
// 引入服务模块
// mod app; // 不再使用独立app构建 / not using standalone app builder
mod cluster;
mod config;
mod domain;
mod net;
mod plugins;
mod router;
mod server;
mod service;
mod storage; // 保留数据结构定义 / Keep data structure definitions
mod tasks;
mod ws;
mod api_registry {
    include!(concat!(env!("OUT_DIR"), "/api_registry.rs"));
}

/// 命令行参数 / Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about = "v-connect-im WebSocket & HTTP Server", long_about = None)]
pub struct Args {
    /// 指定配置文件路径（TOML/JSON/YAML自动识别）
    /// Specify config file path (auto-detect TOML/JSON/YAML)
    #[arg(short = 'c', long = "config", default_value = "config/default.toml")]
    config: Option<String>,
}

// 已通过 pub use 导入作用域 / imported via pub use above

// 连接与服务端结构已迁移至 server 模块 / Connection and server structs moved to server module

impl VConnectIMServer {
    // 构造与配置方法已迁移至 server::VConnectIMServer / Constructors moved to server::VConnectIMServer

    // 离线投递与配额已迁移至 service::offline / Offline delivery moved

    // 离线投递与配额已迁移至 service::offline / Offline quota moved

    // ACK等待与入离线队列已迁移至 service::delivery / Await ACK or queue offline moved

    async fn load_rooms_from_storage(&self) -> Result<usize> {
        // 房间数据已迁移到插件，此方法保留用于兼容性 / Room data migrated to plugin, method kept for compatibility
        // TODO: 从插件加载房间数据 / Load room data from plugin
        Ok(0)
    }

    fn allow_send_to_uid(&self, uid: &str) -> bool {
        if self.blocked_uids.contains(uid) {
            return false;
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        if let Some(mut entry) = self.uid_rate_limits.get_mut(uid) {
            let (limit, count, window_start) = *entry;
            if now_ms - window_start >= 1000 {
                *entry = (limit, 1, now_ms);
                return true;
            } else if count < limit {
                *entry = (limit, count + 1, window_start);
                return true;
            } else {
                return false;
            }
        }
        true
    }

    // HTTP投递方法已迁移至 service::delivery / HTTP delivery moved to service::delivery

    /// HTTP广播消息给所有客户端 / HTTP Broadcast message to all clients
    async fn http_broadcast_message(&self, request: HttpBroadcastRequest) -> HttpBroadcastResponse {
        let wk_msg = ImMessage {
            msg_type: request
                .message_type
                .unwrap_or_else(|| "http_broadcast".to_string()),
            data: serde_json::json!({
                "from": request.from_uid,
                "content": request.content,
                "timestamp": chrono::Utc::now().timestamp_millis()
            }),
            target_uid: None,
        };

        let broadcast_json = match serde_json::to_string(&wk_msg) {
            Ok(json) => json,
            Err(e) => {
                return HttpBroadcastResponse {
                    success: false,
                    message: format!("Failed to serialize broadcast message: {}", e),
                    delivered_count: 0,
                };
            }
        };

        let mut delivered_count = 0;
        let mut failed_clients = Vec::new();

        for entry in self.connections.iter() {
            let client_id = entry.key().clone();

            match self
                .send_message_to_client(&client_id, Message::Text(broadcast_json.clone()))
                .await
            {
                Ok(_) => {
                    delivered_count += 1;
                    debug!("📢 Broadcast message delivered to {}", client_id);
                }
                Err(e) => {
                    debug!("⚠️  Failed to broadcast to {}: {}", client_id, e);
                    failed_clients.push(client_id);
                }
            }
        }

        // 清理失败的连接 / Clean up failed connections
        for client_id in failed_clients {
            self.connections.remove(&client_id);
        }

        info!(
            "📢 HTTP broadcast sent to {} clients ({} failed)",
            delivered_count,
            self.connections.len() - delivered_count
        );

        HttpBroadcastResponse {
            success: delivered_count > 0,
            message: format!("Broadcast delivered to {} clients", delivered_count),
            delivered_count,
        }
    }

    /// HTTP群发到指定房间 / HTTP send group message to room
    async fn http_group_send_message(
        &self,
        room_id: String,
        from_client_id: String,
        content: serde_json::Value,
        message_type: Option<String>,
    ) -> HttpBroadcastResponse {
        let msg_type = message_type.unwrap_or_else(|| "http_group".to_string());
        let message_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();

        // 调用插件系统处理群组消息 / Call plugin system to process group message
        if let Some(pool) = self.plugin_connection_pool.as_ref() {
            let plugin_message = serde_json::json!({
                "message_id": message_id,
                "from_client_id": from_client_id,
                "room_id": room_id,
                "content": content,
                "message_type": msg_type,
                "timestamp": timestamp
            });

            match pool.broadcast_message_event(&plugin_message).await {
                Ok(responses) => {
                    tracing::debug!(
                        "插件处理群组消息响应 / Plugin group message responses: {:?}",
                        responses
                    );
                    // 检查是否有插件要求停止消息传播 / Check if any plugin wants to stop propagation
                    for (_plugin_name, response) in responses {
                        if let Some(flow) = response.get("flow").and_then(|v| v.as_str()) {
                            if flow == "stop" {
                                tracing::info!(
                                    "群组消息被插件拦截 / Group message stopped by plugin"
                                );
                                return HttpBroadcastResponse {
                                    success: false,
                                    message: "Group message blocked by plugin".to_string(),
                                    delivered_count: 0,
                                };
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("插件系统调用失败 / Plugin system call failed: {}", e);
                }
            }
        }

        let forward_msg = ImMessage {
            msg_type: msg_type.clone(),
            data: serde_json::json!({
                "from": from_client_id,
                "room_id": room_id,
                "content": content,
                "timestamp": timestamp,
                "message_id": message_id
            }),
            target_uid: None,
        };
        let forward_json = match serde_json::to_string(&forward_msg) {
            Ok(s) => s,
            Err(e) => {
                return HttpBroadcastResponse {
                    success: false,
                    message: format!("Failed to serialize group message: {}", e),
                    delivered_count: 0,
                }
            }
        };

        let record = storage::MessageRecord {
            message_id: message_id.clone(),
            from_client_id: from_client_id.clone(),
            to_client_id: room_id.clone(),
            content: content.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            msg_type: "group_message".to_string(),
            room_id: Some(room_id.clone()),
        };
        let _ = self.raft.append_entry_as(&self.node_id, &record);
        // storage.append 已移除，使用插件 / storage.append removed, use plugin

        let mut delivered_count = 0usize;
        let mut offline_uids: Vec<String> = Vec::new();
        if let Some(set) = self.rooms.get(&room_id) {
            for uid in set.iter() {
                let uid = uid.clone();
                let clients_opt = self.uid_clients.get(&uid);
                if let Some(clients) = clients_opt {
                    for cid in clients.iter() {
                        let cid = cid.clone();
                        let delivery = if let Some(loc_node) = self.directory.locate_client(&cid) {
                            if loc_node != self.node_id {
                                if let Some(remote) = self.directory.get_server(&loc_node) {
                                    remote
                                        .send_message_to_client(
                                            &cid,
                                            Message::Text(forward_json.clone()),
                                        )
                                        .await
                                } else {
                                    Err(anyhow::anyhow!("remote node not found"))
                                }
                            } else {
                                self.send_message_to_client(
                                    &cid,
                                    Message::Text(forward_json.clone()),
                                )
                                .await
                            }
                        } else {
                            self.send_message_to_client(&cid, Message::Text(forward_json.clone()))
                                .await
                        };
                        if delivery.is_ok() {
                            delivered_count += 1;
                        }
                    }
                } else {
                    offline_uids.push(uid);
                }
            }
        }
        for ou in offline_uids {
            let _ = self.enforce_offline_quota_for_uid(&ou).await;
            let _off = storage::OfflineRecord {
                message_id: message_id.clone(),
                from_uid: self
                    .connections
                    .get(&from_client_id)
                    .and_then(|c| c.uid.clone()),
                to_uid: ou,
                room_id: Some(room_id.clone()),
                content: content.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                msg_type: msg_type.clone(),
            };
            // storage.store_offline 已移除，使用插件 / storage.store_offline removed, use plugin
        }

        HttpBroadcastResponse {
            success: delivered_count > 0,
            message: format!("Group message delivered to {} clients", delivered_count),
            delivered_count,
        }
    }

    /// 获取在线客户端列表 / Get online clients list
    async fn get_online_clients(&self) -> OnlineClientsResponse {
        let mut clients = Vec::new();

        for entry in self.connections.iter() {
            let connection = entry.value();

            if let Ok(last_heartbeat) = connection.last_heartbeat.lock() {
                clients.push(OnlineClientInfo {
                    uid: connection.uid.clone(),
                    addr: connection.addr.to_string(),
                    connected_at: chrono::Utc::now().timestamp_millis(), // 简化处理 / Simplified
                    last_heartbeat: last_heartbeat.elapsed().as_secs() as i64,
                });
            }
        }

        OnlineClientsResponse {
            total_count: clients.len(),
            clients,
        }
    }

    /// 更新客户端心跳 / Update client heartbeat
    async fn update_heartbeat(&self, client_id: &str) {
        if let Some(connection) = self.connections.get(client_id) {
            if let Ok(mut last_heartbeat) = connection.last_heartbeat.lock() {
                *last_heartbeat = Instant::now();
                debug!("💓 Updated heartbeat for client {}", client_id);
            }
        }
    }

    /// 清理超时连接 / Clean up timeout connections
    async fn cleanup_timeout_connections(&self, timeout_ms: u64) {
        let mut disconnected_clients = Vec::new();

        for entry in self.connections.iter() {
            let client_id = entry.key().clone();
            let connection = entry.value();

            if let Ok(last_heartbeat) = connection.last_heartbeat.lock() {
                if last_heartbeat.elapsed().as_millis() > timeout_ms as u128 {
                    disconnected_clients.push(client_id);
                }
            }
        }

        for client_id in disconnected_clients {
            // 主动发送关闭消息 / Send close message proactively
            if let Err(e) = self.send_close_message(&client_id).await {
                error!("Failed to send close message to {}: {}", client_id, e);
            }

            self.connections.remove(&client_id);
            info!("🧹 Cleaned up timeout connection: {}", client_id);
        }
    }

    // 发送/关闭/广播方法已迁移至 ws::sender / send/close/broadcast moved to ws::sender

    async fn handle_incoming_message(
        &self,
        message: Message,
        client_id: &str,
        _connections: &Arc<DashMap<String, Connection>>,
    ) -> Result<()> {
        // 自动更新心跳时间 / Automatically update heartbeat time
        self.update_heartbeat(client_id).await;

        match message {
            Message::Text(text) => {
                debug!("📨 Received text from {}: {}", client_id, text);

                // 尝试解析为JSON消息
                match serde_json::from_str::<ImMessage>(&text) {
                    Ok(mut wk_msg) => {
                        let ctx = PluginContext::new(self, client_id);
                        match self.plugin_registry.emit_incoming(&ctx, &mut wk_msg).await {
                            Ok(PluginFlow::Continue) => {}
                            Ok(PluginFlow::Stop) => {
                                debug!("message blocked by plugin for client {}", client_id);
                                return Ok(());
                            }
                            Err(e) => {
                                error!("plugin incoming error for client {}: {}", client_id, e);
                                return Err(e);
                            }
                        }
                        match wk_msg.msg_type.as_str() {
                            "ping" => {
                                debug!("🏓 Ping from {}", client_id);
                                // 更新心跳时间 / Update heartbeat time
                                self.update_heartbeat(client_id).await;

                                let pong_msg = ImMessage {
                                    msg_type: "pong".to_string(),
                                    data: serde_json::json!({
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                        "client_id": client_id
                                    }),
                                    target_uid: None,
                                };
                                let pong_json = serde_json::to_string(&pong_msg)?;
                                self.send_message_to_client(client_id, Message::Text(pong_json))
                                    .await?;
                            }
                            "online_clients" => {
                                info!("📋 Online clients query from {}", client_id);
                                let online_clients = self.get_online_clients().await;
                                let response_msg = ImMessage {
                                    msg_type: "online_clients_response".to_string(),
                                    data: serde_json::json!(online_clients),
                                    target_uid: None,
                                };
                                let response_json = serde_json::to_string(&response_msg)?;
                                self.send_message_to_client(
                                    client_id,
                                    Message::Text(response_json),
                                )
                                .await?;
                            }
                            "auth" => {
                                info!("🔐 Auth request from {}", client_id);
                                let token = wk_msg
                                    .data
                                    .get("token")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let uid_opt = wk_msg
                                    .data
                                    .get("uid")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                // 优先通过认证插件验证 / Prefer validation via auth plugin
                                let is_valid = if let Some(pool) =
                                    self.plugin_connection_pool.as_ref()
                                {
                                    // 查找认证插件 / Find auth plugin
                                    let auth_plugins: Vec<String> = pool
                                        .list_plugins()
                                        .into_iter()
                                        .filter(|(_, caps)| caps.iter().any(|c| c == "auth"))
                                        .map(|(name, _)| name)
                                        .collect();

                                    if !auth_plugins.is_empty() {
                                        // 调用认证插件验证 token / Call auth plugin to validate token
                                        use prost::Message;
                                        use v::plugin::protocol::{
                                            EventMessage, ValidateTokenRequest,
                                        };

                                        let validate_req = ValidateTokenRequest {
                                            token: token.to_string(),
                                        };

                                        let event = EventMessage {
                                            event_type: "auth.validate_token".to_string(),
                                            payload: validate_req.encode_to_vec(),
                                            timestamp: chrono::Utc::now().timestamp_millis(),
                                            trace_id: client_id.to_string(),
                                        };

                                        match pool.send_event(&auth_plugins[0], &event).await {
                                            Ok(response) => {
                                                // 解析响应 / Parse response
                                                match v::plugin::protocol::ValidateTokenResponse::decode(&response.data[..]) {
                                                    Ok(resp) => {
                                                        info!("✅ 认证插件验证结果 / Auth plugin validation result: valid={}", resp.valid);
                                                        resp.valid
                                                    }
                                                    Err(e) => {
                                                        warn!("认证插件响应解析失败 / Failed to parse auth plugin response: {}", e);
                                                        self.validate_token(token).await.unwrap_or(false)
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("认证插件调用失败，回退到本地验证 / Auth plugin failed, fallback to local: {}", e);
                                                self.validate_token(token).await.unwrap_or(false)
                                            }
                                        }
                                    } else {
                                        // 没有认证插件，使用本地验证 / No auth plugin, use local validation
                                        debug!("未找到认证插件，使用本地验证 / No auth plugin found, using local validation");
                                        self.validate_token(token).await.unwrap_or(false)
                                    }
                                } else {
                                    // 没有插件系统，使用本地验证 / No plugin system, use local validation
                                    self.validate_token(token).await.unwrap_or(false)
                                };
                                let auth_response = ImMessage {
                                    msg_type: "auth_response".to_string(),
                                    data: serde_json::json!({ "status": if is_valid {"success"} else {"failed"}, "message": if is_valid {"Authentication successful"} else {"Authentication failed"} }),
                                    target_uid: None,
                                };
                                let auth_json = serde_json::to_string(&auth_response)?;
                                self.send_message_to_client(client_id, Message::Text(auth_json))
                                    .await?;
                                if is_valid {
                                    if let Some(uid_val) = uid_opt {
                                        // 直接设置 UID / Directly set UID
                                        if let Some(mut conn) = self.connections.get_mut(client_id)
                                        {
                                            conn.uid = Some(uid_val.clone());
                                        }
                                        // 触发认证成功事件 / Emit connection authenticated event
                                        let auth_event = serde_json::json!({
                                            "client_id": client_id,
                                            "uid": uid_val,
                                            "timestamp": chrono::Utc::now().timestamp_millis(),
                                        });
                                        if let Err(e) = self
                                            .plugin_registry
                                            .emit_custom("connection.authenticated", &auth_event)
                                            .await
                                        {
                                            warn!(
                                                "plugin connection.authenticated event error: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            "message" => {
                                info!("💬 Message from {}: {:?}", client_id, wk_msg.data);

                                // 如果有目标ID，发送给指定客户端，否则回声
                                if let Some(target_uid) = &wk_msg.target_uid {
                                    let message_id = Uuid::new_v4().to_string();
                                    let timestamp = chrono::Utc::now().timestamp_millis();
                                    let from_uid = self
                                        .connections
                                        .get(client_id)
                                        .and_then(|c| c.uid.clone())
                                        .unwrap_or_default();

                                    // 调用插件系统处理WebSocket消息 / Call plugin system to process WebSocket message
                                    if let Some(pool) = self.plugin_connection_pool.as_ref() {
                                        let plugin_message = serde_json::json!({
                                            "message_id": message_id,
                                            "from_uid": from_uid,
                                            "to_uid": target_uid,
                                            "content": wk_msg.data,
                                            "message_type": "message",
                                            "timestamp": timestamp
                                        });

                                        match pool.broadcast_message_event(&plugin_message).await {
                                            Ok(responses) => {
                                                tracing::debug!("插件处理WebSocket消息响应 / Plugin WebSocket message responses: {:?}", responses);
                                                // 检查是否有插件要求停止消息传播 / Check if any plugin wants to stop propagation
                                                for (_plugin_name, response) in responses {
                                                    if let Some(flow) = response
                                                        .get("flow")
                                                        .and_then(|v| v.as_str())
                                                    {
                                                        if flow == "stop" {
                                                            tracing::info!("WebSocket消息被插件拦截 / WebSocket message stopped by plugin");
                                                            return Ok(());
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("插件系统调用失败 / Plugin system call failed: {}", e);
                                            }
                                        }
                                    }

                                    let forward_msg = ImMessage {
                                        msg_type: "forwarded_message".to_string(),
                                        data: serde_json::json!({
                                            "from": from_uid,
                                            "content": wk_msg.data,
                                            "timestamp": timestamp,
                                            "message_id": message_id
                                        }),
                                        target_uid: None,
                                    };
                                    let forward_json = serde_json::to_string(&forward_msg)?;
                                    let record = storage::MessageRecord {
                                        message_id: message_id.clone(),
                                        from_client_id: client_id.to_string(),
                                        to_client_id: target_uid.clone(),
                                        content: wk_msg.data.clone(),
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                        msg_type: "message".to_string(),
                                        room_id: None,
                                    };
                                    self.raft.append_entry_as(&self.node_id, &record)?;

                                    // 保存消息到存储插件 / Save message to storage plugin
                                    if let Some(pool) = self.plugin_connection_pool.as_ref() {
                                        let _ = pool
                                            .storage_save_message(
                                                &record.message_id,
                                                &record.from_client_id,
                                                &record.to_client_id,
                                                &record.content,
                                                record.timestamp,
                                                &record.msg_type,
                                                None,
                                            )
                                            .await;
                                    }

                                    // 依据UID发送到所有在线客户端 / deliver to all clients of target uid
                                    let delivery_result = if let Some(clients) =
                                        self.uid_clients.get(target_uid)
                                    {
                                        let mut ok = false;
                                        for cid in clients.iter() {
                                            if self
                                                .send_message_to_client(
                                                    &cid,
                                                    Message::Text(forward_json.clone()),
                                                )
                                                .await
                                                .is_ok()
                                            {
                                                ok = true;
                                            }
                                        }
                                        if ok {
                                            Ok(())
                                        } else {
                                            Err(anyhow::anyhow!("no clients"))
                                        }
                                    } else {
                                        // 跨节点HTTP/RPC转发（最小版本）/ Cross-node forward via HTTP
                                        let mut ok = false;
                                        if let Ok(cm) = v::get_global_config_manager() {
                                            let peers = cm
                                                .get::<String>("cluster.peers")
                                                .unwrap_or_default();
                                            for base in peers
                                                .split(',')
                                                .map(|s| s.trim())
                                                .filter(|s| !s.is_empty())
                                            {
                                                let list_url = format!(
                                                    "{}/v1/internal/clients_by_uid?uid={}",
                                                    base, target_uid
                                                );
                                                if let Ok(resp) = reqwest::get(&list_url).await {
                                                    if resp.status().is_success() {
                                                        if let Ok(val) =
                                                            resp.json::<serde_json::Value>().await
                                                        {
                                                            let ids = val
                                                                .get("client_ids")
                                                                .and_then(|v| v.as_array())
                                                                .cloned()
                                                                .unwrap_or_default();
                                                            if !ids.is_empty() {
                                                                for idv in ids {
                                                                    if let Some(cid) = idv.as_str()
                                                                    {
                                                                        let fwd_url = format!("{}/v1/internal/forward_client", base);
                                                                        let body = serde_json::json!({"client_id": cid, "text": forward_json});
                                                                        if let Ok(res2) =
                                                                            reqwest::Client::new()
                                                                                .post(&fwd_url)
                                                                                .json(&body)
                                                                                .send()
                                                                                .await
                                                                        {
                                                                            if res2
                                                                                .status()
                                                                                .is_success()
                                                                            {
                                                                                ok = true;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                if ok {
                                                    break;
                                                }
                                            }
                                        }
                                        if ok {
                                            Ok(())
                                        } else {
                                            Err(anyhow::anyhow!("uid offline"))
                                        }
                                    };

                                    match delivery_result {
                                        Ok(_) => {
                                            // 同时给发送者确认
                                            let confirm_msg = ImMessage {
                                                msg_type: "message_sent".to_string(),
                                                data: serde_json::json!({
                                                    "to": target_uid,
                                                    "status": "delivered",
                                                    "message_id": message_id
                                                }),
                                                target_uid: None,
                                            };
                                            let confirm_json = serde_json::to_string(&confirm_msg)?;
                                            self.send_message_to_client(
                                                client_id,
                                                Message::Text(confirm_json),
                                            )
                                            .await?;
                                            let deadline_ms = v::get_global_config_manager()
                                                .ok()
                                                .map(|cm| {
                                                    cm.get_or("delivery.deadline_ms", 500_i64)
                                                        as u64
                                                })
                                                .unwrap_or(500);
                                            self.await_ack_or_queue_offline(
                                                target_uid.clone(),
                                                message_id.clone(),
                                                None,
                                                wk_msg.data.clone(),
                                                "message".to_string(),
                                                deadline_ms,
                                            )
                                            .await;
                                        }
                                        Err(_e) => {
                                            return Ok(());
                                        }
                                    }
                                } else {
                                    // 回声给发送者
                                    let echo_msg = ImMessage {
                                        msg_type: "message_echo".to_string(),
                                        data: serde_json::json!({
                                            "original": wk_msg.data,
                                            "from": client_id,
                                            "timestamp": chrono::Utc::now().timestamp_millis()
                                        }),
                                        target_uid: None,
                                    };
                                    let echo_json = serde_json::to_string(&echo_msg)?;
                                    self.send_message_to_client(
                                        client_id,
                                        Message::Text(echo_json),
                                    )
                                    .await?;
                                }
                            }
                            "private_message" => {
                                // 私聊消息，必须有目标ID
                                if let Some(target_uid) = &wk_msg.target_uid {
                                    if !self.allow_send_to_uid(target_uid) {
                                        let err = ImMessage {
                                            msg_type: "error".to_string(),
                                            data: serde_json::json!({"message":"target uid blocked or rate limited"}),
                                            target_uid: None,
                                        };
                                        let txt = serde_json::to_string(&err)?;
                                        self.send_message_to_client(client_id, Message::Text(txt))
                                            .await?;
                                        return Ok(());
                                    }
                                    let message_id = Uuid::new_v4().to_string();
                                    let private_msg = ImMessage {
                                        msg_type: "private_message".to_string(),
                                        data: serde_json::json!({
                                            "from": self.connections.get(client_id).and_then(|c| c.uid.clone()).unwrap_or_default(),
                                            "content": wk_msg.data,
                                            "timestamp": chrono::Utc::now().timestamp_millis(),
                                            "message_id": message_id
                                        }),
                                        target_uid: None,
                                    };
                                    let private_json = serde_json::to_string(&private_msg)?;
                                    let record = storage::MessageRecord {
                                        message_id: message_id.clone(),
                                        from_client_id: client_id.to_string(),
                                        to_client_id: target_uid.clone(),
                                        content: wk_msg.data.clone(),
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                        msg_type: "private_message".to_string(),
                                        room_id: None,
                                    };
                                    self.raft.append_entry_as(&self.node_id, &record)?;
                                    let delivery_result = if let Some(clients) =
                                        self.uid_clients.get(target_uid)
                                    {
                                        let mut ok = false;
                                        for cid in clients.iter() {
                                            if self
                                                .send_message_to_client(
                                                    &cid,
                                                    Message::Text(private_json.clone()),
                                                )
                                                .await
                                                .is_ok()
                                            {
                                                ok = true;
                                            }
                                        }
                                        if ok {
                                            Ok(())
                                        } else {
                                            Err(anyhow::anyhow!("no clients"))
                                        }
                                    } else {
                                        let mut ok = false;
                                        for node in self.directory.list_nodes() {
                                            if let Some(remote) =
                                                self.directory.get_server(&node.node_id)
                                            {
                                                if let Some(rclients) =
                                                    remote.uid_clients.get(target_uid)
                                                {
                                                    for cid in rclients.iter() {
                                                        if remote
                                                            .send_message_to_client(
                                                                &cid,
                                                                Message::Text(private_json.clone()),
                                                            )
                                                            .await
                                                            .is_ok()
                                                        {
                                                            ok = true;
                                                        }
                                                    }
                                                    if ok {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        if ok {
                                            Ok(())
                                        } else {
                                            Err(anyhow::anyhow!("uid offline"))
                                        }
                                    };

                                    match delivery_result {
                                        Ok(_) => {
                                            // 给发送者确认 / Confirm to sender
                                            let confirm_msg = ImMessage {
                                                msg_type: "message_sent".to_string(),
                                                data: serde_json::json!({
                                                    "to": target_uid,
                                                    "status": "delivered",
                                                    "message_id": message_id
                                                }),
                                                target_uid: None,
                                            };
                                            let confirm_json = serde_json::to_string(&confirm_msg)?;
                                            self.send_message_to_client(
                                                client_id,
                                                Message::Text(confirm_json),
                                            )
                                            .await?;
                                            let deadline_ms = v::get_global_config_manager()
                                                .ok()
                                                .map(|cm| {
                                                    cm.get_or("delivery.deadline_ms", 500_i64)
                                                        as u64
                                                })
                                                .unwrap_or(500);
                                            self.await_ack_or_queue_offline(
                                                target_uid.clone(),
                                                message_id.clone(),
                                                None,
                                                wk_msg.data.clone(),
                                                "private_message".to_string(),
                                                deadline_ms,
                                            )
                                            .await;
                                        }
                                        Err(_e) => {
                                            return Ok(());
                                        }
                                    }
                                } else {
                                    let error_msg = ImMessage {
                                        msg_type: "error".to_string(),
                                        data: serde_json::json!({
                                            "message": "private_message requires target_id"
                                        }),
                                        target_uid: None,
                                    };
                                    let error_json = serde_json::to_string(&error_msg)?;
                                    self.send_message_to_client(
                                        client_id,
                                        Message::Text(error_json),
                                    )
                                    .await?;
                                }
                            }
                            "join_room" => {
                                if let Some(room_id) =
                                    wk_msg.data.get("room_id").and_then(|v| v.as_str())
                                {
                                    let uid_opt =
                                        self.connections.get(client_id).and_then(|c| c.uid.clone());
                                    if let Some(uid) = uid_opt {
                                        let set =
                                            self.rooms.entry(room_id.to_string()).or_default();
                                        set.insert(uid.clone());
                                        // 持久化房间成员 / Persist room member
                                        // storage.add_room_member 已移除，使用插件 / storage.add_room_member removed, use plugin
                                        let resp = ImMessage {
                                            msg_type: "join_room_ok".to_string(),
                                            data: serde_json::json!({"room_id": room_id}),
                                            target_uid: None,
                                        };
                                        let txt = serde_json::to_string(&resp)?;
                                        self.send_message_to_client(client_id, Message::Text(txt))
                                            .await?;
                                    } else {
                                        let err = ImMessage {
                                            msg_type: "error".to_string(),
                                            data: serde_json::json!({"message":"join_room requires auth uid"}),
                                            target_uid: None,
                                        };
                                        let txt = serde_json::to_string(&err)?;
                                        self.send_message_to_client(client_id, Message::Text(txt))
                                            .await?;
                                    }
                                }
                            }
                            "leave_room" => {
                                if let Some(room_id) =
                                    wk_msg.data.get("room_id").and_then(|v| v.as_str())
                                {
                                    if let Some(uid) =
                                        self.connections.get(client_id).and_then(|c| c.uid.clone())
                                    {
                                        if let Some(set) = self.rooms.get_mut(room_id) {
                                            set.remove(&uid);
                                        }
                                        // 持久化移除成员 / Persist remove member
                                        // storage.remove_room_member 已移除，使用插件 / storage.remove_room_member removed, use plugin
                                        let resp = ImMessage {
                                            msg_type: "leave_room_ok".to_string(),
                                            data: serde_json::json!({"room_id": room_id}),
                                            target_uid: None,
                                        };
                                        let txt = serde_json::to_string(&resp)?;
                                        self.send_message_to_client(client_id, Message::Text(txt))
                                            .await?;
                                    }
                                }
                            }
                            "group_message" => {
                                let room_id_opt = wk_msg
                                    .data
                                    .get("room_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                if let Some(room_id) = room_id_opt {
                                    let message_id = Uuid::new_v4().to_string();
                                    let forward_msg = ImMessage {
                                        msg_type: "group_message".to_string(),
                                        data: serde_json::json!({
                                            "from": client_id,
                                            "room_id": room_id,
                                            "content": wk_msg.data,
                                            "timestamp": chrono::Utc::now().timestamp_millis(),
                                            "message_id": message_id
                                        }),
                                        target_uid: None,
                                    };
                                    let forward_json = serde_json::to_string(&forward_msg)?;
                                    let record = storage::MessageRecord {
                                        message_id: message_id.clone(),
                                        from_client_id: client_id.to_string(),
                                        to_client_id: room_id.clone(),
                                        content: wk_msg.data.clone(),
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                        msg_type: "group_message".to_string(),
                                        room_id: Some(room_id.clone()),
                                    };
                                    self.raft.append_entry_as(&self.node_id, &record)?;

                                    // 保存消息到存储插件 / Save message to storage plugin
                                    if let Some(pool) = self.plugin_connection_pool.as_ref() {
                                        let _ = pool
                                            .storage_save_message(
                                                &record.message_id,
                                                &record.from_client_id,
                                                &record.to_client_id,
                                                &record.content,
                                                record.timestamp,
                                                &record.msg_type,
                                                record.room_id.as_deref(),
                                            )
                                            .await;
                                    }

                                    let mut delivered_count = 0usize;
                                    let mut offline_uids: Vec<String> = Vec::new();
                                    if let Some(set) = self.rooms.get(&room_id) {
                                        for uid in set.iter() {
                                            let uid = uid.clone();
                                            if !self.allow_send_to_uid(&uid) {
                                                continue;
                                            }
                                            let clients_opt = self.uid_clients.get(&uid);
                                            if let Some(clients) = clients_opt {
                                                for cid in clients.iter() {
                                                    let cid = cid.clone();
                                                    let delivery = if let Some(loc_node) =
                                                        self.directory.locate_client(&cid)
                                                    {
                                                        if loc_node != self.node_id {
                                                            if let Some(remote) =
                                                                self.directory.get_server(&loc_node)
                                                            {
                                                                remote
                                                                    .send_message_to_client(
                                                                        &cid,
                                                                        Message::Text(
                                                                            forward_json.clone(),
                                                                        ),
                                                                    )
                                                                    .await
                                                            } else {
                                                                Err(anyhow::anyhow!(
                                                                    "remote node not found",
                                                                ))
                                                            }
                                                        } else {
                                                            self.send_message_to_client(
                                                                &cid,
                                                                Message::Text(forward_json.clone()),
                                                            )
                                                            .await
                                                        }
                                                    } else {
                                                        self.send_message_to_client(
                                                            &cid,
                                                            Message::Text(forward_json.clone()),
                                                        )
                                                        .await
                                                    };
                                                    if delivery.is_ok() {
                                                        delivered_count += 1;
                                                    }
                                                }
                                            } else {
                                                offline_uids.push(uid);
                                            }
                                        }
                                    } else {
                                        // 回退：从持久化成员列表加载并投递 / Fallback to persistent members
                                        // storage.list_room_members 已移除 / storage.list_room_members removed
                                        if false {
                                            let members: Vec<String> = vec![];
                                            for uid in members {
                                                if !self.allow_send_to_uid(&uid) {
                                                    continue;
                                                }
                                                let clients_opt = self.uid_clients.get(&uid);
                                                if let Some(clients) = clients_opt {
                                                    for cid in clients.iter() {
                                                        let cid = cid.clone();
                                                        let delivery = if let Some(loc_node) =
                                                            self.directory.locate_client(&cid)
                                                        {
                                                            if loc_node != self.node_id {
                                                                if let Some(remote) = self
                                                                    .directory
                                                                    .get_server(&loc_node)
                                                                {
                                                                    remote
                                                                        .send_message_to_client(
                                                                            &cid,
                                                                            Message::Text(
                                                                                forward_json
                                                                                    .clone(),
                                                                            ),
                                                                        )
                                                                        .await
                                                                } else {
                                                                    Err(anyhow::anyhow!(
                                                                        "remote node not found",
                                                                    ))
                                                                }
                                                            } else {
                                                                self.send_message_to_client(
                                                                    &cid,
                                                                    Message::Text(
                                                                        forward_json.clone(),
                                                                    ),
                                                                )
                                                                .await
                                                            }
                                                        } else {
                                                            self.send_message_to_client(
                                                                &cid,
                                                                Message::Text(forward_json.clone()),
                                                            )
                                                            .await
                                                        };
                                                        if delivery.is_ok() {
                                                            delivered_count += 1;
                                                        }
                                                    }
                                                } else {
                                                    offline_uids.push(uid);
                                                }
                                            }
                                        }
                                    }

                                    for ou in offline_uids {
                                        let _ = self.enforce_offline_quota_for_uid(&ou).await;
                                        let _off = storage::OfflineRecord {
                                            message_id: message_id.clone(),
                                            from_uid: self
                                                .connections
                                                .get(client_id)
                                                .and_then(|c| c.uid.clone()),
                                            to_uid: ou,
                                            room_id: Some(room_id.clone()),
                                            content: wk_msg.data.clone(),
                                            timestamp: chrono::Utc::now().timestamp_millis(),
                                            msg_type: "group_message".to_string(),
                                        };
                                        // storage.store_offline 已移除，使用插件 / storage.store_offline removed, use plugin
                                    }

                                    let confirm_msg = ImMessage {
                                        msg_type: "group_message_sent".to_string(),
                                        data: serde_json::json!({
                                            "room_id": room_id,
                                            "delivered_count": delivered_count,
                                            "message_id": message_id
                                        }),
                                        target_uid: None,
                                    };
                                    let confirm_json = serde_json::to_string(&confirm_msg)?;
                                    self.send_message_to_client(
                                        client_id,
                                        Message::Text(confirm_json),
                                    )
                                    .await?;
                                } else {
                                    let error_msg = ImMessage {
                                        msg_type: "error".to_string(),
                                        data: serde_json::json!({
                                            "message": "group_message requires room_id"
                                        }),
                                        target_uid: None,
                                    };
                                    let error_json = serde_json::to_string(&error_msg)?;
                                    self.send_message_to_client(
                                        client_id,
                                        Message::Text(error_json),
                                    )
                                    .await?;
                                }
                            }
                            "ack" => {
                                // 客户端确认消息ID（按UID）/ Client acknowledges message ID (by uid)
                                if let Some(msg_id) =
                                    wk_msg.data.get("message_id").and_then(|v| v.as_str())
                                {
                                    let uid_key = self
                                        .connections
                                        .get(client_id)
                                        .and_then(|c| c.uid.clone())
                                        .unwrap_or_default();
                                    if !uid_key.is_empty() {
                                        let set = self.acked_ids.entry(uid_key).or_default();
                                        set.insert(msg_id.to_string());
                                        debug!("✅ Ack received from uid for {}", msg_id);
                                    }
                                }
                            }
                            _ => {
                                warn!(
                                    "⚠️  Unknown message type from {}: {}",
                                    client_id, wk_msg.msg_type
                                );
                                let error_msg = ImMessage {
                                    msg_type: "error".to_string(),
                                    data: serde_json::json!({
                                        "message": format!("Unknown message type: {}", wk_msg.msg_type)
                                    }),
                                    target_uid: None,
                                };
                                let error_json = serde_json::to_string(&error_msg)?;
                                self.send_message_to_client(client_id, Message::Text(error_json))
                                    .await?;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  Invalid JSON from {}: {}", client_id, e);
                        let error_msg = ImMessage {
                            msg_type: "error".to_string(),
                            data: serde_json::json!({
                                "message": "Invalid JSON format"
                            }),
                            target_uid: None,
                        };
                        let error_json = serde_json::to_string(&error_msg)?;
                        self.send_message_to_client(client_id, Message::Text(error_json))
                            .await?;
                    }
                }
            }
            Message::Binary(data) => {
                debug!(
                    "📦 Received binary data from {}: {} bytes",
                    client_id,
                    data.len()
                );
                // 回声二进制数据给发送者
                self.send_message_to_client(client_id, Message::Binary(data))
                    .await?;
            }
            Message::Ping(_data) => {
                debug!("🏓 Received ping from {}", client_id);
                // 自动处理pong在tokio-tungstenite中
            }
            Message::Pong(_) => {
                debug!("🏸 Received pong from {}", client_id);
            }
            Message::Close(frame) => {
                info!("🔒 Client {} requested close: {:?}", client_id, frame);
            }
            _ => {
                debug!("❓ Received other message type from {}", client_id);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn replicate_record(&self, rec: &storage::MessageRecord) -> Result<()> {
        self.raft.append_entry_as(&self.node_id, rec)?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn replicate_with_retry(
        &self,
        rec: &storage::MessageRecord,
        max_retries: u32,
        backoff_ms: u64,
    ) -> Result<()> {
        let mut attempt = 0;
        loop {
            if self.raft.append_entry_as(&self.node_id, rec).is_ok() {
                return Ok(());
            }
            if attempt >= max_retries {
                return Err(anyhow::anyhow!("replication quorum not met"));
            }
            attempt += 1;
            tokio::time::sleep(Duration::from_millis(backoff_ms * attempt as u64)).await;
        }
    }

    /// 验证令牌（允许本地/远端）/ Validate token (local/remote)
    async fn validate_token(&self, token: &str) -> Result<bool> {
        if token.is_empty() {
            return Ok(false);
        }
        if let Some(cfg) = &self.auth_config {
            if !cfg.enabled {
                // 关闭鉴权时可选允许测试令牌 / Allow test token when auth disabled
                return Ok(true);
            }
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(cfg.timeout_ms))
                .build()?;
            let resp = client
                .get(format!("{}/v1/sso/auth", cfg.center_url))
                .query(&[("token", token)])
                .send()
                .await?;
            Ok(resp.status().is_success())
        } else {
            Ok(true)
        }
    }
} // impl VConnectIMServer 结束 / End of impl VConnectIMServer

// 为IM服务器实现统一健康检查接口
// Implement unified HealthCheck for IM server
// HealthCheck 的具体实现已迁移至 service::health

// Clone 已在 server 模块实现 / Clone implemented in server module

fn build_http_server(
    server: Arc<VConnectIMServer>,
    host: String,
    port: u16,
) -> Result<actix_web::dev::Server> {
    let addr = format!("{}:{}", host, port);
    // 启动前打印路由映射（自动生成） / Print auto-generated route map before start
    api_registry::print_routes(&addr, &["Logger"]);

    // 使用 actix-web 构建路由（自动注册） / Build routes with actix-web (auto registry)
    let actix = HttpServer::new(move || {
        App::new()
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add(("Access-Control-Allow-Headers", "*"))
                    .add((
                        "Access-Control-Allow-Methods",
                        "GET, POST, PUT, DELETE, OPTIONS",
                    )),
            )
            .app_data(web::Data::new(server.clone()))
            .configure(|cfg| {
                crate::api::openapi::register(cfg, "/openapi.json");
            })
            // 只保留健康检查接口 / Only keep health check endpoints
            .configure(crate::router::configure)
    })
    .bind(addr.clone())?
    .disable_signals()
    .shutdown_timeout(30);

    info!("🌐 HTTP Server starting on http://{}", addr);

    Ok(actix.run())
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志 / Initialize logging
    init_tracing();

    let args = Args::parse();

    info!("🎯 Starting v-connect-im Hybrid Server (WebSocket + HTTP)...");

    // 如果提供配置文件路径则使用之，否则加载本服务默认配置
    // Initialize global config with provided file or service default
    if let Some(cfg_path) = &args.config {
        v::init_global_config_with_file(cfg_path)?;
        info!("🔧 Loaded config file: {}", cfg_path);
    } else {
        let default_cfg = format!("{}/config/default.toml", env!("CARGO_MANIFEST_DIR"));
        v::init_global_config_with_file(&default_cfg)?;
        info!("🔧 Loaded default config: {}", default_cfg);
    }

    // 打印配置源信息（已初始化后） / Print sources after init
    let cm = v::get_global_config_manager()?;
    cm.print_sources_info();

    // 读取配置项 / Read configuration items
    let cm = v::get_global_config_manager()?;
    let host: String = cm.get_or("server.host", "127.0.0.1".to_string());
    let ws_port: u16 = cm.get_or("server.ws_port", 5200_i64) as u16;
    let http_port: u16 = cm.get_or("server.http_port", 8080_i64) as u16;
    let timeout_ms: u64 = cm.get_or("server.timeout_ms", 10000_i64) as u64;

    // 鉴权配置 / Auth Configuration
    let auth_enabled: bool = cm.get_or("auth.enabled", false);
    let auth_center_url: String = cm.get_or("auth.center_url", "http://127.0.0.1:8090".to_string());
    let auth_timeout_ms: u64 = cm.get_or("auth.timeout_ms", 1000_i64) as u64;

    // 插件安装配置 / Plugin installation configuration
    let plugin_dir: String = cm.get_or("plugins.plugin_dir", "./plugins".to_string());
    let plugin_install_urls: Vec<String> =
        cm.get::<Vec<String>>("plugins.install").unwrap_or_default();

    // 创建服务器 / Create server
    let mut server_builder = VConnectIMServer::new();
    if auth_enabled {
        let auth_cfg = crate::config::AuthConfigLite {
            enabled: auth_enabled,
            center_url: auth_center_url,
            timeout_ms: auth_timeout_ms,
        };
        server_builder = server_builder.with_auth_config(auth_cfg);
    }
    // 初始化并安装插件 / Initialize and install plugins
    if !plugin_install_urls.is_empty() {
        use v::plugin::installer::PluginInstaller;
        let installer = PluginInstaller::new(&plugin_dir);
        if let Err(e) = installer.init() {
            warn!("Failed to initialize plugin directory: {}", e);
        } else {
            info!(
                "📦 Installing plugins from {} URL(s)",
                plugin_install_urls.len()
            );
            for url in plugin_install_urls {
                match installer.install_from_url(&url).await {
                    Ok(name) => {
                        info!("✅ Plugin installed: {}", name);
                    }
                    Err(e) => {
                        error!("❌ Failed to install plugin from {}: {}", url, e);
                    }
                }
            }
        }
    }

    // 初始化插件运行时管理器 / Initialize plugin runtime manager
    use crate::plugins::runtime::PluginRuntimeManager;
    let socket_dir = format!("{}/sockets", plugin_dir);
    let mut runtime_manager = PluginRuntimeManager::new(&plugin_dir, &socket_dir);

    // 读取插件 debug 配置 / Read plugin debug configuration
    let plugin_debug: bool = cm.get_or("plugins.debug", false);
    let plugin_log_level: Option<String> = cm.get::<String>("plugins.log_level").ok();

    if plugin_debug {
        runtime_manager.set_debug_mode(true);
        info!("🐛 Plugin debug mode enabled");
    }

    if let Some(ref level) = plugin_log_level {
        runtime_manager.set_log_level(level.clone());
        info!("📊 Plugin log level: {}", level);
    }

    if let Err(e) = runtime_manager.init() {
        warn!("Failed to initialize plugin runtime manager: {}", e);
    } else {
        info!("🔌 Plugin runtime manager initialized");
    }

    // 注册开发模式插件 / Register development mode plugins
    let dev_plugins: Vec<String> = cm
        .get::<Vec<String>>("plugins.dev_plugins")
        .unwrap_or_default();

    for dev_plugin in dev_plugins {
        if let Some((name, path)) = dev_plugin.split_once(':') {
            let path = std::path::PathBuf::from(path);
            if path.exists() {
                if let Err(e) = runtime_manager.register_dev_plugin(name.to_string(), path.clone())
                {
                    warn!("Failed to register dev plugin {}: {}", name, e);
                } else {
                    info!("🛠️ Registered dev plugin: {} from {}", name, path.display());
                }
            } else {
                warn!("Dev plugin path not found: {}", path.display());
            }
        } else {
            warn!("Invalid dev_plugin format: {}", dev_plugin);
        }
    }

    // 启动 Unix Socket 服务器（若未配置则自动生成路径）/ Start Unix Socket server (auto generate path if absent)
    let socket_path = cm
        .get::<String>("plugins.socket_path")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| format!("{}/sockets/runtime.sock", plugin_dir));

    // 展开 ~ 为用户主目录 / Expand ~ to user home directory
    let socket_path = if socket_path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = std::path::Path::new(&home);
            home_path
                .join(&socket_path[2..])
                .to_string_lossy()
                .to_string()
        } else {
            socket_path
        }
    } else {
        socket_path
    };

    runtime_manager.set_global_socket_path(&socket_path);
    let runtime_manager_arc = Arc::new(runtime_manager);
    // 全局关闭通道（供各子系统共享）/ Global shutdown channel for subsystems
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    use crate::plugins::runtime::UnixSocketServer;
    let (socket_server_task, plugin_connection_pool) = match UnixSocketServer::new(
        &socket_path,
        runtime_manager_arc.clone(),
        shutdown_rx.clone(),
    )
    .await
    {
        Ok(server) => {
            info!("🔌 Unix Socket server starting on: {}", socket_path);
            let pool = server.connection_pool();
            let task = Some(tokio::spawn(async move {
                if let Err(e) = server.run().await {
                    error!("Unix Socket server error: {}", e);
                }
            }));
            (task, Some(pool))
        }
        Err(e) => {
            warn!("Failed to start Unix Socket server: {}", e);
            (None, None)
        }
    };

    // 启动所有已安装插件（确保 socket 已经监听）/ Start installed plugins after socket ready
    {
        let rm = runtime_manager_arc.clone();
        match rm.start_all().await {
            Ok(_) => info!("🚀 All plugins started"),
            Err(e) => warn!("Failed to start plugins: {}", e),
        }
    }

    let node_id: String = cm.get_or("server.node_id", "node-local".to_string());
    let directory = Arc::new(cluster::directory::Directory::new());
    let raft_cluster = Arc::new(cluster::raft::RaftCluster::new(
        directory.clone(),
        node_id.clone(),
    ));
    server_builder = server_builder.with_node(node_id.clone(), directory.clone());
    server_builder = server_builder.with_raft(raft_cluster.clone());
    server_builder = server_builder.with_plugin_runtime_manager(runtime_manager_arc.clone());
    if let Some(ref pool) = plugin_connection_pool {
        server_builder = server_builder.with_plugin_connection_pool(pool.clone());
    }
    let server = Arc::new(server_builder);
    directory.register_server(&node_id, server.clone());
    if let Err(e) = server.plugin_registry.emit_startup(server.as_ref()).await {
        warn!("plugin startup error: {}", e);
    }
    let plugin_cfg = serde_json::json!({
        "plugins": {}
    });
    server.set_plugin_config(plugin_cfg.clone());
    if let Err(e) = server.plugin_registry.emit_config_update(&plugin_cfg).await {
        warn!("plugin config update error: {}", e);
    }

    // 加载持久化房间成员到内存
    let _ = server.load_rooms_from_storage().await;

    let server_clone = server.clone();
    let server_http = server.clone();

    tasks::heartbeat::spawn_cleanup_task(server_clone, timeout_ms, shutdown_rx.clone());

    // 启动WebSocket服务器 / Start WebSocket server
    let ws_server = server.clone();
    let ws_host = host.clone();
    let mut ws_shutdown_rx = shutdown_rx.clone();
    let ws_future = async move {
        info!("🚀 Starting WebSocket server on {}:{}", ws_host, ws_port);
        tokio::select! {
            res = ws_server.run(ws_host, ws_port) => {
                if let Err(e) = res { error!("❌ WebSocket server error: {}", e); }
            }
            _ = ws_shutdown_rx.changed() => {
                info!("🛑 WebSocket shutdown signal received");
            }
        }
    };

    let http_host = host.clone();
    let mut http_shutdown_rx = shutdown_rx.clone();
    let http_future = async move {
        sleep(Duration::from_secs(1)).await;
        match build_http_server(server_http, http_host.clone(), http_port) {
            Ok(server) => {
                let handle = server.handle();
                tokio::pin!(server);
                loop {
                    tokio::select! {
                        res = &mut server => {
                            if let Err(e) = res { error!("❌ HTTP server error: {}", e); }
                            break;
                        }
                        _ = http_shutdown_rx.changed() => {
                            info!("🛑 HTTP shutdown signal received");
                            let _ = handle.stop(true).await;
                        }
                    }
                }
            }
            Err(e) => {
                error!("❌ HTTP server build error: {}", e);
            }
        }
    };

    // 等待服务器运行 / Wait for servers to run
    let socket_task = socket_server_task;
    tokio::select! {
        _ = ws_future => {
            info!("WebSocket server stopped");
            let _ = shutdown_tx.send(true);
        }
        _ = http_future => {
            info!("HTTP server stopped");
            let _ = shutdown_tx.send(true);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("🛎️ Ctrl-C received, initiating shutdown");
            let _ = shutdown_tx.send(true);
        }
    }

    // 等待 Unix Socket server 任务完成 / Wait for Unix Socket server task to complete
    if let Some(handle) = socket_task {
        info!("⏳ 等待 Unix Socket server 退出 / Waiting for Unix Socket server to exit");
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(_) => {
                info!("✅ Unix Socket server 已退出 / Unix Socket server exited");
            }
            Err(_) => {
                warn!("⏰ Unix Socket server 退出超时 / Unix Socket server exit timeout");
            }
        }
    }

    // 关闭所有插件连接 / Close all plugin connections
    if let Some(pool) = &plugin_connection_pool {
        pool.close_all().await;
    }

    // 停止所有插件 / Stop all plugins
    debug!("🛑 开始停止所有插件 / Starting to stop all plugins");
    if let Err(e) = runtime_manager_arc.stop_all().await {
        warn!("Failed to stop plugins: {}", e);
    }
    debug!("✅ 所有插件已停止 / All plugins stopped");

    debug!("📢 发送插件关闭事件 / Emitting plugin shutdown event");
    if let Err(e) = server.plugin_registry.emit_shutdown().await {
        warn!("plugin shutdown error: {}", e);
    }
    debug!("✅ 插件关闭事件已发送 / Plugin shutdown event emitted");

    info!("✅ Server shutdown successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Message;

    #[tokio::test]
    async fn test_ping_pong_and_private_message_ack() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        let (a_tx, mut a_rx) = mpsc::unbounded_channel::<Message>();
        let (b_tx, mut b_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        let b_id = "B".to_string();

        server.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some(a_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server.connections.insert(
            b_id.clone(),
            Connection {
                client_id: b_id.clone(),
                uid: Some(b_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: b_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server
            .uid_clients
            .entry(a_id.clone())
            .or_default()
            .insert(a_id.clone());
        server
            .uid_clients
            .entry(b_id.clone())
            .or_default()
            .insert(b_id.clone());

        // ping
        let ping = ImMessage {
            msg_type: "ping".to_string(),
            data: serde_json::json!({}),
            target_uid: None,
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&ping).unwrap()),
                &a_id,
                &server.connections,
            )
            .await
            .unwrap();
        let pong = a_rx.recv().await.unwrap();
        let pong_text = match pong {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let pong_msg: ImMessage = serde_json::from_str(&pong_text).unwrap();
        assert_eq!(pong_msg.msg_type, "pong");

        // private message
        let pm = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"hello"}),
            target_uid: Some(b_id.clone()),
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server.connections,
            )
            .await
            .unwrap();

        let b_msg = b_rx.recv().await.unwrap();
        let b_text = match b_msg {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let b_wk: ImMessage = serde_json::from_str(&b_text).unwrap();
        assert_eq!(b_wk.msg_type, "private_message");
        let message_id = b_wk
            .data
            .get("message_id")
            .and_then(|v| v.as_str())
            .expect("message_id");

        let a_confirm = a_rx.recv().await.unwrap();
        let a_text = match a_confirm {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let a_wk: ImMessage = serde_json::from_str(&a_text).unwrap();
        assert_eq!(a_wk.msg_type, "message_sent");
        assert_eq!(
            a_wk.data
                .get("message_id")
                .and_then(|v| v.as_str())
                .unwrap(),
            message_id
        );

        // ack
        let ack = ImMessage {
            msg_type: "ack".to_string(),
            data: serde_json::json!({"message_id": message_id}),
            target_uid: None,
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&ack).unwrap()),
                &a_id,
                &server.connections,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_cross_node_private_message_routing() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut a_builder = VConnectIMServer::new();
        a_builder = a_builder
            .with_node("node-A".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_a = Arc::new(a_builder);
        directory.register_server("node-A", server_a.clone());

        let mut b_builder = VConnectIMServer::new();
        b_builder = b_builder
            .with_node("node-B".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_b = Arc::new(b_builder);
        directory.register_server("node-B", server_b.clone());

        let (a_tx, mut a_rx) = mpsc::unbounded_channel::<Message>();
        let (b_tx, mut b_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        let b_id = "B".to_string();

        server_a.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some(a_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server_b.connections.insert(
            b_id.clone(),
            Connection {
                client_id: b_id.clone(),
                uid: Some(b_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: b_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");
        directory.register_client_location(&b_id, "node-B");
        server_a
            .uid_clients
            .entry(a_id.clone())
            .or_default()
            .insert(a_id.clone());
        server_b
            .uid_clients
            .entry(b_id.clone())
            .or_default()
            .insert(b_id.clone());
        server_a
            .uid_clients
            .entry(a_id.clone())
            .or_default()
            .insert(a_id.clone());
        server_b
            .uid_clients
            .entry(b_id.clone())
            .or_default()
            .insert(b_id.clone());
        server_a
            .uid_clients
            .entry(a_id.clone())
            .or_default()
            .insert(a_id.clone());
        server_b
            .uid_clients
            .entry(b_id.clone())
            .or_default()
            .insert(b_id.clone());

        let pm = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"cross"}),
            target_uid: Some(b_id.clone()),
        };
        server_a
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server_a.connections,
            )
            .await
            .unwrap();

        let b_msg = b_rx.recv().await.unwrap();
        let b_text = match b_msg {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let b_wk: ImMessage = serde_json::from_str(&b_text).unwrap();
        assert_eq!(b_wk.msg_type, "private_message");

        let a_confirm = a_rx.recv().await.unwrap();
        let a_text = match a_confirm {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let a_wk: ImMessage = serde_json::from_str(&a_text).unwrap();
        assert_eq!(a_wk.msg_type, "message_sent");
    }

    #[tokio::test]
    async fn test_storage_append_on_private_message() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut s1b = VConnectIMServer::new();
        s1b = s1b
            .with_node("node-A".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_a = Arc::new(s1b);
        directory.register_server("node-A", server_a.clone());

        let mut s2b = VConnectIMServer::new();
        s2b = s2b
            .with_node("node-B".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_b = Arc::new(s2b);
        directory.register_server("node-B", server_b.clone());

        let (a_tx, mut a_rx) = mpsc::unbounded_channel::<Message>();
        let (b_tx, mut b_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        let b_id = "B".to_string();

        server_a.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some(a_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server_b.connections.insert(
            b_id.clone(),
            Connection {
                client_id: b_id.clone(),
                uid: Some(b_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: b_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");
        directory.register_client_location(&b_id, "node-B");
        server_a
            .uid_clients
            .entry(a_id.clone())
            .or_default()
            .insert(a_id.clone());
        server_b
            .uid_clients
            .entry(b_id.clone())
            .or_default()
            .insert(b_id.clone());

        let pm = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"persist"}),
            target_uid: Some(b_id.clone()),
        };
        server_a
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server_a.connections,
            )
            .await
            .unwrap();

        let b_msg = b_rx.recv().await.unwrap();
        let b_text = match b_msg {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let b_wk: ImMessage = serde_json::from_str(&b_text).unwrap();
        let message_id = b_wk
            .data
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        let stored_local = server_a.storage.get(&message_id).unwrap();
        let stored_remote = server_b.storage.get(&message_id).unwrap();
        assert!(stored_local.is_some());
        assert!(stored_remote.is_some());

        let _ = a_rx.recv().await.unwrap(); // confirm
    }

    #[tokio::test]
    async fn test_replication_retry_failure() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut a_builder = VConnectIMServer::new();
        a_builder = a_builder
            .with_node("node-A".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_a = Arc::new(a_builder);
        directory.register_server("node-A", server_a.clone());

        // 注册一个无Server的节点以提高quorum / register node without server to increase quorum
        directory.register_node(cluster::router::NodeInfo {
            node_id: "node-B".into(),
            weight: 1,
            is_alive: true,
        });

        let (a_tx, mut _a_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        server_a.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some(a_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");

        let pm = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"fail"}),
            target_uid: Some("B".into()),
        };
        let result = server_a
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server_a.connections,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_leader_switch_and_write() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut a_builder = VConnectIMServer::new();
        a_builder = a_builder
            .with_node("node-A".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_a = Arc::new(a_builder);
        directory.register_server("node-A", server_a.clone());

        let mut b_builder = VConnectIMServer::new();
        b_builder = b_builder
            .with_node("node-B".to_string(), directory.clone())
            .with_raft(raft.clone());
        let server_b = Arc::new(b_builder);
        directory.register_server("node-B", server_b.clone());

        let (a_tx, mut _a_rx) = mpsc::unbounded_channel::<Message>();
        let (b_tx, mut _b_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        let b_id = "B".to_string();
        server_a.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some(a_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server_b.connections.insert(
            b_id.clone(),
            Connection {
                client_id: b_id.clone(),
                uid: Some(b_id.clone()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: b_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");
        directory.register_client_location(&b_id, "node-B");

        let pm = ImMessage {
            msg_type: "private_message".into(),
            data: serde_json::json!({"text":"first"}),
            target_uid: Some(b_id.clone()),
        };
        raft.set_leader("node-A".into());
        // Leader为A时，A写入成功
        let res_ok = server_a
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server_a.connections,
            )
            .await;
        assert!(res_ok.is_ok());

        // 切换Leader到B
        raft.set_leader("node-B".into());
        let pm2 = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"second"}),
            target_uid: Some(a_id.clone()),
        };
        // A再写入应失败
        let res_err = server_a
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm2).unwrap()),
                &a_id,
                &server_a.connections,
            )
            .await;
        assert!(res_err.is_err());
        // B写入应成功
        let res_ok_b = server_b
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm2).unwrap()),
                &b_id,
                &server_b.connections,
            )
            .await;
        assert!(res_ok_b.is_ok());
    }

    #[cfg(feature = "raft_async")]
    #[tokio::test]
    async fn test_async_raft_three_nodes_election_and_replication() {
        use crate::cluster::raft_async::AsyncRaftCluster;
        let cluster = AsyncRaftCluster::new();
        // 3 节点添加 / add 3 nodes
        cluster.add_node("node-1", storage::Storage::open_temporary().unwrap());
        cluster.add_node("node-2", storage::Storage::open_temporary().unwrap());
        cluster.add_node("node-3", storage::Storage::open_temporary().unwrap());
        // 选举 / elect leader
        cluster.elect("node-2");
        // 写入并复制 / write and replicate
        let rec = storage::MessageRecord {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_client_id: "A".into(),
            to_client_id: "B".into(),
            content: serde_json::json!({"text":"raft-async"}),
            timestamp: chrono::Utc::now().timestamp_millis(),
            msg_type: "private_message".into(),
            room_id: None,
        };
        let ok = cluster.write("node-2", &rec).await;
        assert!(ok.is_ok());
        // 非Leader写入失败 / non-leader write fails
        let err = cluster.write("node-1", &rec).await;
        assert!(err.is_err());

        // 安装快照 / install snapshot
        cluster
            .install_snapshot_from_leader("/tmp/raft-async-snap")
            .unwrap();
    }

    #[tokio::test]
    async fn test_group_message_and_offline_store() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        // Online client A with uid uA
        let (a_tx, mut a_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        server.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some("uA".to_string()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");
        // Map uid->client
        server
            .uid_clients
            .entry("uA".to_string())
            .or_default()
            .insert(a_id.clone());

        // Room r1 with members uA (online) and uB (offline)
        server
            .rooms
            .entry("r1".to_string())
            .or_default()
            .insert("uA".to_string());
        server
            .rooms
            .entry("r1".to_string())
            .or_default()
            .insert("uB".to_string());

        // Send group message via WS handler
        let gm = ImMessage {
            msg_type: "group_message".to_string(),
            data: serde_json::json!({"room_id":"r1","text":"hi"}),
            target_uid: None,
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&gm).unwrap()),
                &a_id,
                &server.connections,
            )
            .await
            .unwrap();

        // A should receive group_message and confirmation
        let first = a_rx.recv().await.unwrap();
        let first_text = match first {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let wk1: ImMessage = serde_json::from_str(&first_text).unwrap();
        assert_eq!(wk1.msg_type, "group_message");
        let second = a_rx.recv().await.unwrap();
        let second_text = match second {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let wk2: ImMessage = serde_json::from_str(&second_text).unwrap();
        assert_eq!(wk2.msg_type, "group_message_sent");

        // Offline pull for uB should contain one record
        let list = server.storage.pull_offline("uB", 10).unwrap();
        assert!(list.len() >= 1);
        assert_eq!(list[0].to_uid, "uB");
        assert_eq!(list[0].room_id.as_deref(), Some("r1"));

        // Ack offline
        let removed = server
            .storage
            .ack_offline("uB", &[list[0].message_id.clone()])
            .unwrap();
        assert_eq!(removed, 1);
    }

    #[tokio::test]
    async fn test_load_rooms_from_storage() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        // 预写入持久化成员
        server.storage.add_room_member("r1", "uA").unwrap();
        server.storage.add_room_member("r1", "uB").unwrap();
        server.storage.add_room_member("r2", "uC").unwrap();

        // 清空内存并加载
        server.rooms.clear();
        let total = server.load_rooms_from_storage().await.unwrap();
        assert!(total >= 3);
        let r1 = server.rooms.get("r1").unwrap();
        assert!(r1.contains("uA"));
        assert!(r1.contains("uB"));
        let r2 = server.rooms.get("r2").unwrap();
        assert!(r2.contains("uC"));
    }

    #[tokio::test]
    async fn test_offline_time_filter_pagination() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        let uid = "uT";
        let base = chrono::Utc::now().timestamp_millis();
        let rec1 = crate::storage::OfflineRecord {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_uid: Some("uA".into()),
            to_uid: uid.into(),
            room_id: Some("r1".into()),
            content: serde_json::json!({"n":1}),
            timestamp: base - 2000,
            msg_type: "group_message".into(),
        };
        let rec2 = crate::storage::OfflineRecord {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_uid: Some("uA".into()),
            to_uid: uid.into(),
            room_id: Some("r1".into()),
            content: serde_json::json!({"n":2}),
            timestamp: base - 1000,
            msg_type: "group_message".into(),
        };
        let rec3 = crate::storage::OfflineRecord {
            message_id: uuid::Uuid::new_v4().to_string(),
            from_uid: Some("uA".into()),
            to_uid: uid.into(),
            room_id: Some("r1".into()),
            content: serde_json::json!({"n":3}),
            timestamp: base,
            msg_type: "group_message".into(),
        };
        server.storage.store_offline(&rec1).unwrap();
        server.storage.store_offline(&rec2).unwrap();
        server.storage.store_offline(&rec3).unwrap();

        // 过滤时间窗口 [base-1500, base-500]
        let (items, next) = server
            .storage
            .pull_offline_by_time(uid, None, 10, Some(base - 1500), Some(base - 500))
            .unwrap();
        assert_eq!(items.len(), 1); // 只有rec2
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_group_message_fallback_persist_members() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        // 持久化房间成员，但不在内存 / Persist members but not in memory
        server.storage.add_room_member("rP", "uX").unwrap();
        server.rooms.clear();

        // 在线客户端映射 uid->client
        let (x_tx, mut x_rx) = mpsc::unbounded_channel::<Message>();
        let x_id = "X".to_string();
        server.connections.insert(
            x_id.clone(),
            Connection {
                client_id: x_id.clone(),
                uid: Some("uX".to_string()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: x_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&x_id, "node-A");
        server
            .uid_clients
            .entry("uX".to_string())
            .or_default()
            .insert(x_id.clone());

        // 发送群聊消息到 rP，应当通过持久化成员路由到 X
        let gm = ImMessage {
            msg_type: "group_message".to_string(),
            data: serde_json::json!({"room_id":"rP","text":"fallback"}),
            target_uid: None,
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&gm).unwrap()),
                &x_id,
                &server.connections,
            )
            .await
            .unwrap();

        let recv = x_rx.recv().await.unwrap();
        let txt = match recv {
            Message::Text(t) => t,
            _ => panic!("expected text"),
        };
        let wk: ImMessage = serde_json::from_str(&txt).unwrap();
        assert_eq!(wk.msg_type, "group_message");
    }

    #[tokio::test]
    async fn test_offline_cleanup_and_members_pagination() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        // 离线清理
        let uid = "uC";
        let now = chrono::Utc::now().timestamp_millis();
        for i in 0..5 {
            let rec = crate::storage::OfflineRecord {
                message_id: uuid::Uuid::new_v4().to_string(),
                from_uid: Some("uA".into()),
                to_uid: uid.into(),
                room_id: Some("rC".into()),
                content: serde_json::json!({"i":i}),
                timestamp: now - (i * 1000) as i64,
                msg_type: "group_message".into(),
            };
            server.storage.store_offline(&rec).unwrap();
        }
        let removed = server.storage.cleanup_offline(uid, now - 2500, 10).unwrap();
        assert!(removed >= 2);

        // 成员分页
        for u in ["u1", "u2", "u3", "u4", "u5"].iter() {
            server.storage.add_room_member("rM", u).unwrap();
        }
        let (page1, cur1) = server
            .storage
            .list_room_members_paginated("rM", Some("u"), None, 2)
            .unwrap();
        assert_eq!(page1.len(), 2);
        let (page2, _cur2) = server
            .storage
            .list_room_members_paginated("rM", Some("u"), cur1, 2)
            .unwrap();
        assert_eq!(page2.len(), 2);
    }

    #[tokio::test]
    async fn test_ack_deadline_queues_offline() {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-A".into(),
        ));
        let mut builder = VConnectIMServer::new();
        builder = builder
            .with_node("node-A".into(), directory.clone())
            .with_raft(raft.clone());
        let server = Arc::new(builder);
        directory.register_server("node-A", server.clone());

        let (a_tx, mut _a_rx) = mpsc::unbounded_channel::<Message>();
        let (b_tx, mut b_rx) = mpsc::unbounded_channel::<Message>();
        let a_id = "A".to_string();
        let b_id = "B".to_string();
        server.connections.insert(
            a_id.clone(),
            Connection {
                client_id: a_id.clone(),
                uid: Some("uA".to_string()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: a_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        server.connections.insert(
            b_id.clone(),
            Connection {
                client_id: b_id.clone(),
                uid: Some("uB".to_string()),
                addr: "127.0.0.1:0".parse().unwrap(),
                sender: b_tx,
                last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
            },
        );
        directory.register_client_location(&a_id, "node-A");
        directory.register_client_location(&b_id, "node-A");
        server
            .uid_clients
            .entry("uA".to_string())
            .or_default()
            .insert(a_id.clone());
        server
            .uid_clients
            .entry("uB".to_string())
            .or_default()
            .insert(b_id.clone());

        let pm = ImMessage {
            msg_type: "private_message".to_string(),
            data: serde_json::json!({"text":"no-ack"}),
            target_uid: Some("uB".to_string()),
        };
        server
            .handle_incoming_message(
                Message::Text(serde_json::to_string(&pm).unwrap()),
                &a_id,
                &server.connections,
            )
            .await
            .unwrap();
        let _delivered = b_rx.recv().await.unwrap();

        tokio::time::sleep(Duration::from_millis(600)).await;
        // 应有离线数据 / should be queued offline
        let count = server.storage.offline_count("uB").unwrap();
        assert!(count >= 1);
    }
}
// 为兼容现有API文件的导入，导出常用类型 / Re-export common types for API compatibility
// 底部重复导出移除 / remove duplicated bottom re-exports
// 对外导出常用类型，兼容已有API的 `use crate::...` 导入 / Re-export commonly used types for API files compatibility
pub use crate::domain::message::{
    ConnectRequest, ConnectResponse, HttpBroadcastRequest, HttpBroadcastResponse,
    HttpSendMessageRequest, HttpSendMessageResponse, ImMessage, OnlineClientInfo,
    OnlineClientsResponse, WebhookClientStatusData, WebhookEvent, WebhookEventType,
    WebhookMessageData,
};
pub use crate::server::{Connection, VConnectIMServer};
