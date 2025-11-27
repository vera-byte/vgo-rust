use crate::plugins::{
    bridge::HttpBridgePlugin, sensitive::SensitiveWordPlugin, trace::TracePlugin, PluginContext,
    PluginFlow,
};
use actix_web::dev::Service;
use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::Parser;
use dashmap::{DashMap, DashSet};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::future::ready;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{accept_async, tungstenite::Message};
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
mod storage;
mod tasks;
mod ws;
mod api_registry {
    include!(concat!(env!("OUT_DIR"), "/api_registry.rs"));
}
//

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
        let rooms = self.storage.list_rooms()?;
        let mut total = 0usize;
        for rid in rooms {
            let members = self.storage.list_room_members(&rid)?;
            let set = self.rooms.entry(rid).or_default();
            for u in members {
                set.insert(u);
                total += 1;
            }
        }
        Ok(total)
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
        let forward_msg = ImMessage {
            msg_type: msg_type.clone(),
            data: serde_json::json!({
                "from": from_client_id,
                "room_id": room_id,
                "content": content,
                "timestamp": chrono::Utc::now().timestamp_millis(),
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
        let _ = self.storage.append(&record);

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
            let off = storage::OfflineRecord {
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
            let _ = self.storage.store_offline(&off);
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

    // run 方法已迁移至 ws::server::run / run method moved to ws::server::run

    async fn handle_connection(
        stream: TcpStream,
        peer_addr: SocketAddr,
        connections: Arc<DashMap<String, Connection>>,
        server: VConnectIMServer,
    ) -> Result<()> {
        info!("📨 New connection from: {}", peer_addr);

        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // 创建通道用于向该客户端发送消息
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // 生成唯一客户端ID
        let client_id = Uuid::new_v4().to_string();

        // 启动消息发送任务
        let client_id_clone = client_id.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let is_close = matches!(&msg, Message::Close(_));
                if let Err(e) = ws_sender.send(msg).await {
                    error!("Failed to send message to {}: {}", client_id_clone, e);
                    break;
                }
                if is_close {
                    let _ = ws_sender.close().await;
                    break;
                }
            }
        });

        // 创建连接信息 / Create connection info
        let connection = Connection {
            client_id: client_id.clone(),
            uid: None,
            addr: peer_addr,
            sender: tx,
            last_heartbeat: Arc::new(std::sync::Mutex::new(Instant::now())),
        };

        // 存储连接
        connections.insert(client_id.clone(), connection);
        server
            .directory
            .register_client_location(&client_id, &server.node_id);

        info!("✅ Client {} connected from {}", client_id, peer_addr);

        // 触发连接建立事件 / Emit connection established event
        let conn_event = serde_json::json!({
            "client_id": client_id,
            "addr": peer_addr.to_string(),
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        if let Err(e) = server
            .plugin_registry
            .emit_custom("connection.established", &conn_event)
            .await
        {
            warn!("plugin connection.established event error: {}", e);
        }

        // 发送客户端上线Webhook事件 / Send client online webhook event
        service::webhook::send_client_online_webhook(&server, &client_id, &None, &peer_addr).await;
        let welcome_text = "Welcome to v-connect-im Server".to_string();

        // 发送欢迎消息 / Send welcome message
        let welcome_msg = ConnectResponse {
            status: "connected".to_string(),
            message: welcome_text,
        };

        server
            .send_message_to_client(
                &client_id,
                Message::Text(serde_json::to_string(&welcome_msg)?),
            )
            .await?;

        // 授权看门狗：连接后必须在deadline内鉴权，否则踢出 / Auth watchdog: require auth within deadline or disconnect
        let auth_deadline_ms: u64 = v::get_global_config_manager()
            .ok()
            .map(|cm| cm.get_or("auth.deadline_ms", 1000_u64))
            .unwrap_or(1000);
        {
            let watchdog_client = client_id.clone();
            let watchdog_connections = connections.clone();
            let watchdog_server = server.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(auth_deadline_ms)).await;
                if let Some(conn) = watchdog_connections.get(&watchdog_client) {
                    if conn.uid.is_none() {
                        let _ = watchdog_server.send_close_message(&watchdog_client).await;
                        watchdog_connections.remove(&watchdog_client);
                        tracing::warn!(
                            "disconnecting unauthenticated client_id={}",
                            watchdog_client
                        );
                    }
                }
            });
        }

        // 处理来自该客户端的消息
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(message) => {
                    if let Err(e) = server
                        .handle_incoming_message(message, &client_id, &connections)
                        .await
                    {
                        error!("Error handling message from {}: {}", client_id, e);
                    }
                }
                Err(e) => {
                    error!("WebSocket error from {}: {}", client_id, e);
                    break;
                }
            }
        }

        // 清理
        let connection_info = connections.remove(&client_id);
        send_task.abort();
        info!("👋 Client {} disconnected", client_id);

        // 触发连接关闭事件 / Emit connection closed event
        let close_event = serde_json::json!({
            "client_id": client_id,
            "addr": peer_addr.to_string(),
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        if let Err(e) = server
            .plugin_registry
            .emit_custom("connection.closed", &close_event)
            .await
        {
            warn!("plugin connection.closed event error: {}", e);
        }

        // 发送客户端离线Webhook事件 / Send client offline webhook event
        if let Some((_, connection)) = connection_info {
            let connected_at = chrono::Utc::now().timestamp_millis()
                - connection
                    .last_heartbeat
                    .lock()
                    .unwrap()
                    .elapsed()
                    .as_millis() as i64;
            service::webhook::send_client_offline_webhook(
                &server,
                &client_id,
                &connection.uid,
                &connection.addr,
                connected_at,
            )
            .await;

            if let Some(uid) = &connection.uid {
                let mut is_last_connection = false;
                if let Some(set) = server.uid_clients.get_mut(uid) {
                    set.remove(&client_id);
                    is_last_connection = set.is_empty();
                }
                // 如果是最后一个连接，触发 user.offline 事件 / Emit user.offline if last connection
                if is_last_connection {
                    let event = serde_json::json!({
                        "uid": uid,
                        "client_id": client_id,
                        "timestamp": chrono::Utc::now().timestamp_millis(),
                    });
                    if let Err(e) = server
                        .plugin_registry
                        .emit_custom("user.offline", &event)
                        .await
                    {
                        warn!("plugin user.offline event error: {}", e);
                    }
                }
            }
        }

        Ok(())
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
                                let is_valid = self
                                    .auth_plugin
                                    .as_ref()
                                    .validate(self, token)
                                    .await
                                    .unwrap_or(false);
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
                                        let _ = self
                                            .auth_plugin
                                            .as_ref()
                                            .apply(self, client_id, &uid_val)
                                            .await;
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
                                    {
                                        let message_id = Uuid::new_v4().to_string();
                                        let forward_msg = ImMessage {
                                            msg_type: "forwarded_message".to_string(),
                                            data: serde_json::json!({
                                                "from": self.connections.get(client_id).and_then(|c| c.uid.clone()).unwrap_or_default(),
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
                                            to_client_id: target_uid.clone(),
                                            content: wk_msg.data.clone(),
                                            timestamp: chrono::Utc::now().timestamp_millis(),
                                            msg_type: "message".to_string(),
                                            room_id: None,
                                        };
                                        self.raft.append_entry_as(&self.node_id, &record)?;
                                        let _ = self.storage.append(&record);
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
                                                    if let Ok(resp) = reqwest::get(&list_url).await
                                                    {
                                                        if resp.status().is_success() {
                                                            if let Ok(val) = resp
                                                                .json::<serde_json::Value>()
                                                                .await
                                                            {
                                                                let ids = val
                                                                    .get("client_ids")
                                                                    .and_then(|v| v.as_array())
                                                                    .cloned()
                                                                    .unwrap_or_default();
                                                                if !ids.is_empty() {
                                                                    for idv in ids {
                                                                        if let Some(cid) =
                                                                            idv.as_str()
                                                                        {
                                                                            let fwd_url = format!("{}/v1/internal/forward_client", base);
                                                                            let body = serde_json::json!({"client_id": cid, "text": forward_json});
                                                                            if let Ok(res2) = reqwest::Client::new().post(&fwd_url).json(&body).send().await {
                                                                                if res2.status().is_success() { ok = true; }
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
                                                // 发送消息送达Webhook事件 / Send message delivered webhook event
                                                service::webhook::send_message_webhook(
                                                    self,
                                                    &message_id,
                                                    client_id,
                                                    &None, // from_uid
                                                    &Some(target_uid.clone()),
                                                    &Some(target_uid.clone()),
                                                    &wk_msg.data,
                                                    "message",
                                                    "delivered",
                                                    Some(chrono::Utc::now().timestamp_millis()),
                                                )
                                                .await;

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
                                                let confirm_json =
                                                    serde_json::to_string(&confirm_msg)?;
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
                                                // 发送消息失败Webhook事件 / Send message failed webhook event
                                                let message_id = Uuid::new_v4().to_string();
                                                service::webhook::send_message_webhook(
                                                    self,
                                                    &message_id,
                                                    client_id,
                                                    &None, // from_uid
                                                    &Some(target_uid.clone()),
                                                    &Some(target_uid.clone()),
                                                    &wk_msg.data,
                                                    "message",
                                                    "failed",
                                                    None,
                                                )
                                                .await;
                                                return Ok(());
                                            }
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
                                            // 发送私聊消息送达Webhook事件 / Send private message delivered webhook event
                                            service::webhook::send_message_webhook(
                                                self,
                                                &message_id,
                                                client_id,
                                                &self
                                                    .connections
                                                    .get(client_id)
                                                    .and_then(|c| c.uid.clone()),
                                                &None,
                                                &Some(target_uid.clone()),
                                                &wk_msg.data,
                                                "private_message",
                                                "delivered",
                                                Some(chrono::Utc::now().timestamp_millis()),
                                            )
                                            .await;

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
                                            // 发送私聊消息失败Webhook事件 / Send private message failed webhook event
                                            service::webhook::send_message_webhook(
                                                self,
                                                &message_id,
                                                client_id,
                                                &None, // from_uid
                                                &Some(target_uid.clone()),
                                                &Some(target_uid.clone()),
                                                &wk_msg.data,
                                                "private_message",
                                                "failed",
                                                None,
                                            )
                                            .await;
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
                                        let _ = self.storage.add_room_member(room_id, &uid);
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
                                        let _ = self.storage.remove_room_member(room_id, &uid);
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
                                        if let Ok(members) =
                                            self.storage.list_room_members(&room_id)
                                        {
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
                                        let off = storage::OfflineRecord {
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
                                        let _ = self.storage.store_offline(&off);
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

    /// 发送Webhook事件 / Send Webhook Event
    async fn send_webhook_event(&self, event_type: WebhookEventType, data: serde_json::Value) {
        if let Some(webhook_config) = &self.webhook_config {
            if !webhook_config.enabled {
                return;
            }

            let event = WebhookEvent {
                event_type: event_type.clone(),
                event_id: Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                data,
                retry_count: Some(0),
            };

            let webhook_config = webhook_config.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::deliver_webhook_event(webhook_config, event).await {
                    error!("❌ Failed to deliver webhook event: {}", e);
                }
            });
        }
    }

    /// 交付Webhook事件到第三方服务器 / Deliver Webhook Event to Third-party Server
    async fn deliver_webhook_event(
        webhook_config: crate::config::WebhookConfigLite,
        event: WebhookEvent,
    ) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(webhook_config.timeout_ms))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let url = webhook_config
            .url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Webhook url not configured"))?;
        let mut request = client.post(url).json(&event);

        // 添加签名头 / Add signature header if secret is configured
        if let Some(secret) = &webhook_config.secret {
            let signature = Self::generate_webhook_signature(&event, secret);
            request = request.header("X-VConnectIM-Signature", signature);
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Webhook request failed: {}", e))?;

        if response.status().is_success() {
            info!("✅ Webhook event {} delivered successfully", event.event_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Webhook delivery failed with status {}: {}",
                status,
                body
            ))
        }
    }

    /// 生成Webhook签名 / Generate Webhook Signature
    fn generate_webhook_signature(event: &WebhookEvent, secret: &str) -> String {
        use std::collections::HashMap;

        let mut data = HashMap::new();
        data.insert("event_id", event.event_id.as_str());

        let event_type_str = format!("{:?}", event.event_type);
        data.insert("event_type", event_type_str.as_str());

        let timestamp_str = event.timestamp.to_string();
        data.insert("timestamp", timestamp_str.as_str());

        let payload = serde_json::to_string(&data).unwrap_or_default();

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());

        let result = mac.finalize();
        let code_bytes = result.into_bytes();

        format!("sha256={}", hex::encode(code_bytes))
    }

    /// 发送客户端上线Webhook事件 / Send Client Online Webhook Event
    #[allow(dead_code)]
    async fn send_client_online_webhook(
        &self,
        client_id: &str,
        uid: &Option<String>,
        addr: &SocketAddr,
    ) {
        let data = serde_json::json!(WebhookClientStatusData {
            client_id: client_id.to_string(),
            uid: uid.clone(),
            addr: addr.to_string(),
            connected_at: Some(chrono::Utc::now().timestamp_millis()),
            disconnected_at: None,
            online_duration_ms: None,
        });

        self.send_webhook_event(WebhookEventType::ClientOnline, data)
            .await;
    }

    /// 发送客户端离线Webhook事件 / Send Client Offline Webhook Event
    #[allow(dead_code)]
    async fn send_client_offline_webhook(
        &self,
        client_id: &str,
        uid: &Option<String>,
        addr: &SocketAddr,
        connected_at: i64,
    ) {
        let now = chrono::Utc::now().timestamp_millis();
        let online_duration_ms = (now - connected_at).max(0) as u64;

        let data = serde_json::json!(WebhookClientStatusData {
            client_id: client_id.to_string(),
            uid: uid.clone(),
            addr: addr.to_string(),
            connected_at: Some(connected_at),
            disconnected_at: Some(now),
            online_duration_ms: Some(online_duration_ms),
        });

        self.send_webhook_event(WebhookEventType::ClientOffline, data)
            .await;
    }

    /// 发送消息Webhook事件 / Send Message Webhook Event
    async fn send_message_webhook(
        &self,
        message_id: &str,
        from_client_id: &str,
        from_uid: &Option<String>,
        to_client_id: &Option<String>,
        to_uid: &Option<String>,
        content: &serde_json::Value,
        message_type: &str,
        delivery_status: &str,
        delivered_at: Option<i64>,
    ) {
        let data = serde_json::json!(WebhookMessageData {
            message_id: message_id.to_string(),
            from_client_id: from_client_id.to_string(),
            from_uid: from_uid.clone(),
            to_client_id: to_client_id.clone(),
            to_uid: to_uid.clone(),
            content: content.clone(),
            message_type: message_type.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            delivered_at,
            delivery_status: delivery_status.to_string(),
        });

        let event_type = match delivery_status {
            "delivered" => WebhookEventType::MessageDelivered,
            "failed" => WebhookEventType::MessageFailed,
            _ => WebhookEventType::MessageSent,
        };

        self.send_webhook_event(event_type, data).await;
    }
}

// 为IM服务器实现统一健康检查接口
// Implement unified HealthCheck for IM server
// HealthCheck 的具体实现已迁移至 service::health

// Clone 已在 server 模块实现 / Clone implemented in server module

/// 启动HTTP服务器 / Start HTTP server
async fn start_http_server(server: Arc<VConnectIMServer>, host: String, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    // 启动前打印路由映射（自动生成） / Print auto-generated route map before start
    api_registry::print_routes(&addr, &["Logger"]);

    // 使用 actix-web 构建路由（自动注册） / Build routes with actix-web (auto registry)
    HttpServer::new(move || {
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
            .configure(crate::api::plugins::configure)
            .configure(|cfg| {
                crate::api::openapi::register(cfg, "/openapi.json");
            })
            // 内部跨节点API / internal cross-node APIs
            .configure(|cfg| {
                crate::api::v1::internal::has_client::register(cfg, "/v1/internal/has_client");
                crate::api::v1::internal::forward_client::register(
                    cfg,
                    "/v1/internal/forward_client",
                );
                crate::api::v1::internal::clients_by_uid::register(
                    cfg,
                    "/v1/internal/clients_by_uid",
                );
            })
            .configure(crate::router::configure)
    })
    .bind(addr.clone())?
    .run()
    .await?;

    info!("🌐 HTTP Server starting on http://{}", addr);
    info!("📡 Available HTTP endpoints:");
    info!("   POST /api/send - Send message to specific client");
    info!("   POST /api/broadcast - Broadcast message to all clients");
    info!("   GET  /health - Basic health check");
    info!("   GET  /health/detailed - Detailed health check with metrics");
    info!("   GET  /health/ready - Readiness check");
    info!("   GET  /health/live - Liveness check");
    info!("");
    info!("💡 Example HTTP requests:");
    info!("   Send: curl -X POST http://{}/api/send -H 'Content-Type: application/json' -d '{{\"from_client_id\":\"xxx\",\"to_client_id\":\"yyy\",\"content\":{{\"text\":\"Hello\"}}}}'", addr);
    info!("   Broadcast: curl -X POST http://{}/api/broadcast -H 'Content-Type: application/json' -d '{{\"from_client_id\":\"xxx\",\"content\":{{\"text\":\"Hello All\"}}}}'", addr);
    info!("   Health: curl http://{}/health", addr);
    info!("   Detailed Health: curl http://{}/health/detailed", addr);

    Ok(())
}

// 处理CORS预检请求（全路径匹配）/ Handle CORS preflight for all paths
async fn preflight_ok() -> actix_web::HttpResponse {
    actix_web::HttpResponse::NoContent()
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .insert_header(("Access-Control-Allow-Headers", "*"))
        .insert_header((
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        ))
        .finish()
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

    let webhook_url: Option<String> = cm.get::<String>("webhook.url").ok();
    let webhook_timeout_ms: u64 = cm.get_or("webhook.timeout_ms", 3000000_i64) as u64;
    let webhook_secret: Option<String> = cm.get::<String>("webhook.secret").ok();
    let webhook_enabled: bool = cm.get_or("webhook.enabled", false);
    let trace_enabled: bool = cm.get_or("plugins.trace_enabled", 0_i64) == 1;
    let trace_log_payload: bool = cm.get_or("plugins.trace_log_payload", 0_i64) == 1;
    let bridge_enabled: bool = cm.get_or("plugins.bridge_enabled", 0_i64) == 1;
    let bridge_callback_timeout_ms: u64 =
        cm.get_or("plugins.bridge_callback_timeout_ms", 1000_i64) as u64;
    let sensitive_words: Vec<String> = cm
        .get::<Vec<String>>("plugins.sensitive_words")
        .unwrap_or_default();

    // Webhook配置 / Webhook Configuration
    if webhook_enabled && webhook_url.is_some() {
        info!("📡 Webhook Configuration:");
        info!("   URL: {}", webhook_url.as_ref().unwrap());
        info!("   Timeout: {}ms", webhook_timeout_ms);
        info!(
            "   Secret: {}",
            if webhook_secret.is_some() {
                "Configured"
            } else {
                "None"
            }
        );
    } else {
        info!("📡 Webhook: Disabled");
    }

    info!("");
    info!("📖 WebSocket message types:");
    info!("   - ping: Heartbeat (with automatic heartbeat tracking)");
    info!("   - auth: Authentication");
    info!("   - message: Send message with optional target_id");
    info!("   - private_message: Send private message (requires target_id)");
    info!("   - online_clients: Query online clients list");
    info!("");
    info!("💡 WebSocket examples:");
    info!("   Ping: {{\"type\":\"ping\",\"data\":{{}}}}");
    info!("   Auth: {{\"type\":\"auth\",\"data\":{{\"uid\":\"test\",\"token\":\"token\"}}}}");
    info!("   Message: {{\"type\":\"message\",\"data\":{{\"content\":\"Hello\"}},\"target_id\":\"client_id\"}}");
    info!("   Private: {{\"type\":\"private_message\",\"data\":{{\"content\":\"Hello\"}},\"target_id\":\"client_id\"}}");
    info!("   Online Clients: {{\"type\":\"online_clients\",\"data\":{{}}}}");

    // 创建带Webhook配置的服务器 / Create server with webhook configuration
    let mut server_builder = VConnectIMServer::new();
    if webhook_enabled && webhook_url.is_some() {
        let webhook_config = crate::config::WebhookConfigLite {
            url: Some(webhook_url.unwrap()),
            timeout_ms: webhook_timeout_ms,
            secret: webhook_secret,
            enabled: true,
        };
        server_builder = server_builder.with_webhook_config(webhook_config);
    }
    if auth_enabled {
        let auth_cfg = crate::config::AuthConfigLite {
            enabled: auth_enabled,
            center_url: auth_center_url,
            timeout_ms: auth_timeout_ms,
        };
        server_builder = server_builder.with_auth_config(auth_cfg);
    }
    if trace_enabled {
        info!(
            "🧩 Trace plugin enabled (payload_log={})",
            trace_log_payload
        );
        server_builder = server_builder.with_plugin(Arc::new(TracePlugin::new(trace_log_payload)));
    }
    if bridge_enabled {
        match HttpBridgePlugin::new(
            server_builder.remote_plugins.clone(),
            bridge_callback_timeout_ms,
        ) {
            Ok(plugin) => {
                info!(
                    "🧩 HTTP bridge plugin enabled (timeout={}ms)",
                    bridge_callback_timeout_ms
                );
                server_builder = server_builder.with_plugin(Arc::new(plugin));
            }
            Err(e) => {
                warn!("failed to init http bridge plugin: {}", e);
            }
        }
    }
    let sensitive_plugin = Arc::new(SensitiveWordPlugin::new(sensitive_words.clone()));
    server_builder = server_builder.with_plugin(sensitive_plugin);
    let node_id: String = cm.get_or("server.node_id", "node-local".to_string());
    let directory = Arc::new(cluster::directory::Directory::new());
    let raft_cluster = Arc::new(cluster::raft::RaftCluster::new(
        directory.clone(),
        node_id.clone(),
    ));
    server_builder = server_builder.with_node(node_id.clone(), directory.clone());
    server_builder = server_builder.with_raft(raft_cluster.clone());
    let server = Arc::new(server_builder);
    directory.register_server(&node_id, server.clone());
    if let Err(e) = server.plugin_registry.emit_startup(server.as_ref()).await {
        warn!("plugin startup error: {}", e);
    }
    let plugin_cfg = serde_json::json!({
        "plugins": {
            "trace_enabled": trace_enabled,
            "trace_log_payload": trace_log_payload,
            "bridge_enabled": bridge_enabled,
            "bridge_callback_timeout_ms": bridge_callback_timeout_ms,
            "sensitive_words": sensitive_words,
        }
    });
    server.set_plugin_config(plugin_cfg.clone());
    if let Err(e) = server.plugin_registry.emit_config_update(&plugin_cfg).await {
        warn!("plugin config update error: {}", e);
    }

    // 加载持久化房间成员到内存
    let _ = server.load_rooms_from_storage().await;

    let server_clone = server.clone();
    let server_http = server.clone();

    // 启动自动心跳清理任务 / Start automatic heartbeat cleanup task
    tasks::heartbeat::spawn_cleanup_task(server_clone, timeout_ms);

    // 启动WebSocket服务器 / Start WebSocket server
    let ws_server = server.clone();
    let ws_host = host.clone();
    let ws_future = async move {
        info!("🚀 Starting WebSocket server on {}:{}", ws_host, ws_port);
        if let Err(e) = ws_server.run(ws_host, ws_port).await {
            error!("❌ WebSocket server error: {}", e);
        }
    };

    // 启动HTTP服务器 / Start HTTP server
    let http_host = host.clone();
    let http_future = async move {
        // 等待WebSocket服务器启动 / Wait for WebSocket server to start
        sleep(Duration::from_secs(1)).await;
        info!("🌐 Starting HTTP server on {}:{}", http_host, http_port);
        if let Err(e) = start_http_server(server_http, http_host, http_port).await {
            error!("❌ HTTP server error: {}", e);
        }
    };

    // 等待两个服务器运行 / Wait for both servers to run
    tokio::select! {
        _ = ws_future => {
            info!("WebSocket server stopped");
        }
        _ = http_future => {
            info!("HTTP server stopped");
        }
    }

    if let Err(e) = server.plugin_registry.emit_shutdown().await {
        warn!("plugin shutdown error: {}", e);
    }

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
