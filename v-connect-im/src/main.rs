use actix_web::{web, App, HttpServer};
use anyhow::Result;
use async_trait::async_trait; // 异步Trait支持 / Async trait support
use clap::Parser;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
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
mod service;
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

/// 悟空消息结构 / WuKong Message Structure
/// 支持中英文消息类型 / Supports both Chinese and English message types
#[derive(Serialize, Deserialize, Debug)]
pub struct WuKongMessage {
    #[serde(rename = "type")]
    msg_type: String,
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_id: Option<String>, // 目标客户端ID / Target client ID
}

/// 连接请求 / Connection Request
#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectRequest {
    uid: String,
    token: String,
}

/// 连接响应 / Connection Response
#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectResponse {
    status: String,
    message: String,
    client_id: String,
}

/// 在线客户端信息 / Online Client Information
#[derive(Serialize, Deserialize, Debug)]
pub struct OnlineClientInfo {
    client_id: String,
    uid: Option<String>,
    addr: String,
    connected_at: i64,
    last_heartbeat: i64,
}

/// 在线客户端列表响应 / Online Clients List Response
#[derive(Serialize, Deserialize, Debug)]
pub struct OnlineClientsResponse {
    clients: Vec<OnlineClientInfo>,
    total_count: usize,
}

/// HTTP发送消息请求 / HTTP Send Message Request
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpSendMessageRequest {
    from_client_id: String,       // 发送者客户端ID / Sender client ID
    to_client_id: String,         // 接收者客户端ID / Receiver client ID
    content: serde_json::Value,   // 消息内容 / Message content
    message_type: Option<String>, // 消息类型 / Message type
}

/// HTTP发送消息响应 / HTTP Send Message Response
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpSendMessageResponse {
    success: bool,              // 是否成功 / Success flag
    message: String,            // 响应消息 / Response message
    message_id: Option<String>, // 消息ID / Message ID
    delivered_at: Option<i64>,  // 送达时间 / Delivery time
}

/// HTTP广播消息请求 / HTTP Broadcast Message Request
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpBroadcastRequest {
    from_client_id: String,       // 发送者客户端ID / Sender client ID
    content: serde_json::Value,   // 消息内容 / Message content
    message_type: Option<String>, // 消息类型 / Message type
}

/// HTTP广播消息响应 / HTTP Broadcast Message Response
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpBroadcastResponse {
    success: bool,          // 是否成功 / Success flag
    message: String,        // 响应消息 / Response message
    delivered_count: usize, // 送达数量 / Delivery count
}

/// Webhook事件类型 / Webhook Event Types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
enum WebhookEventType {
    ClientOnline,     // 客户端上线 / Client online
    ClientOffline,    // 客户端离线 / Client offline
    MessageSent,      // 消息发送 / Message sent
    MessageDelivered, // 消息送达 / Message delivered
    MessageFailed,    // 消息发送失败 / Message failed
}

/// Webhook事件数据 / Webhook Event Data
#[derive(Serialize, Deserialize, Debug)]
pub struct WebhookEvent {
    event_type: WebhookEventType, // 事件类型 / Event type
    event_id: String,             // 事件唯一ID / Event unique ID
    timestamp: i64,               // 事件时间戳（毫秒）/ Event timestamp (milliseconds)
    data: serde_json::Value,      // 事件数据 / Event data
    retry_count: Option<u32>,     // 重试次数 / Retry count
}

/// Webhook配置 / Webhook Configuration
#[derive(Clone)]
pub struct WebhookConfig {
    url: String,            // Webhook URL
    timeout_ms: u64,        // 超时时间（毫秒）/ Timeout (milliseconds)
    secret: Option<String>, // 签名密钥 / Signature secret
    enabled: bool,          // 是否启用 / Whether enabled
}

/// Webhook客户端状态数据 / Webhook Client Status Data
#[derive(Serialize, Deserialize, Debug)]
pub struct WebhookClientStatusData {
    client_id: String,               // 客户端ID / Client ID
    uid: Option<String>,             // 用户ID / User ID
    addr: String,                    // 客户端地址 / Client address
    connected_at: Option<i64>,       // 连接时间 / Connection time
    disconnected_at: Option<i64>,    // 断开时间 / Disconnection time
    online_duration_ms: Option<u64>, // 在线时长（毫秒）/ Online duration (milliseconds)
}

/// Webhook消息数据 / Webhook Message Data
#[derive(Serialize, Deserialize, Debug)]
pub struct WebhookMessageData {
    message_id: String,           // 消息ID / Message ID
    from_client_id: String,       // 发送者客户端ID / Sender client ID
    from_uid: Option<String>,     // 发送者用户ID / Sender user ID
    to_client_id: Option<String>, // 接收者客户端ID / Receiver client ID
    to_uid: Option<String>,       // 接收者用户ID / Receiver user ID
    content: serde_json::Value,   // 消息内容 / Message content
    message_type: String,         // 消息类型 / Message type
    timestamp: i64,               // 消息时间戳（毫秒）/ Message timestamp (milliseconds)
    delivered_at: Option<i64>,    // 送达时间（毫秒）/ Delivery time (milliseconds)
    delivery_status: String,      // 送达状态 / Delivery status
}

/// 客户端连接信息 / Client Connection Information
#[derive(Clone)]
pub struct Connection {
    client_id: String,                              // 客户端唯一ID / Client unique ID
    uid: Option<String>,                            // 用户ID / User ID
    addr: SocketAddr,                               // 客户端地址 / Client address
    sender: mpsc::UnboundedSender<Message>,         // 消息发送器 / Message sender
    last_heartbeat: Arc<std::sync::Mutex<Instant>>, // 最后心跳时间 / Last heartbeat time
}

/// 服务端全局状态 / Server Global State
pub struct VConnectIMServer {
    connections: Arc<DashMap<String, Connection>>, // 客户端连接 / Client connections
    webhook_config: Option<WebhookConfig>,         // Webhook配置 / Webhook configuration
}

impl VConnectIMServer {
    fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            webhook_config: None,
        }
    }

    fn with_webhook_config(mut self, config: WebhookConfig) -> Self {
        self.webhook_config = Some(config);
        self
    }

    /// HTTP发送消息给指定客户端 / HTTP Send message to specific client
    async fn http_send_message(&self, request: HttpSendMessageRequest) -> HttpSendMessageResponse {
        let message_id = Uuid::new_v4().to_string();
        let delivered_at = chrono::Utc::now().timestamp_millis();

        // 构建消息 / Build message
        let message_type = request
            .message_type
            .clone()
            .unwrap_or_else(|| "http_message".to_string());
        let wk_msg = WuKongMessage {
            msg_type: message_type.clone(),
            data: serde_json::json!({
                "from": request.from_client_id,
                "content": request.content,
                "timestamp": delivered_at,
                "message_id": &message_id
            }),
            target_id: None, // 消息已经指定了目标 / Message already has target
        };

        // 发送消息 / Send message
        let forward_json = match serde_json::to_string(&wk_msg) {
            Ok(json) => json,
            Err(e) => {
                return HttpSendMessageResponse {
                    success: false,
                    message: format!("Failed to serialize message: {}", e),
                    message_id: None,
                    delivered_at: None,
                };
            }
        };

        match self
            .send_message_to_client(&request.to_client_id, Message::Text(forward_json))
            .await
        {
            Ok(_) => {
                info!(
                    "📤 HTTP message sent from {} to {}",
                    request.from_client_id, request.to_client_id
                );

                // 发送消息送达Webhook事件 / Send message delivered webhook event
                self.send_message_webhook(
                    &message_id,
                    &request.from_client_id,
                    &None, // from_uid - would need to be tracked
                    &Some(request.to_client_id.clone()),
                    &None, // to_uid - would need to be tracked
                    &request.content,
                    &message_type,
                    "delivered",
                    Some(delivered_at),
                )
                .await;

                HttpSendMessageResponse {
                    success: true,
                    message: "Message delivered successfully".to_string(),
                    message_id: Some(message_id),
                    delivered_at: Some(delivered_at),
                }
            }
            Err(e) => {
                warn!("⚠️  Failed to send HTTP message: {}", e);

                // 发送消息失败Webhook事件 / Send message failed webhook event
                self.send_message_webhook(
                    &message_id,
                    &request.from_client_id,
                    &None,
                    &Some(request.to_client_id.clone()),
                    &None,
                    &request.content,
                    &message_type,
                    "failed",
                    None,
                )
                .await;

                HttpSendMessageResponse {
                    success: false,
                    message: format!("Failed to deliver message: {}", e),
                    message_id: Some(message_id),
                    delivered_at: None,
                }
            }
        }
    }

    /// HTTP广播消息给所有客户端 / HTTP Broadcast message to all clients
    async fn http_broadcast_message(&self, request: HttpBroadcastRequest) -> HttpBroadcastResponse {
        let wk_msg = WuKongMessage {
            msg_type: request
                .message_type
                .unwrap_or_else(|| "http_broadcast".to_string()),
            data: serde_json::json!({
                "from": request.from_client_id,
                "content": request.content,
                "timestamp": chrono::Utc::now().timestamp_millis()
            }),
            target_id: None,
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

    /// 获取在线客户端列表 / Get online clients list
    async fn get_online_clients(&self) -> OnlineClientsResponse {
        let mut clients = Vec::new();

        for entry in self.connections.iter() {
            let client_id = entry.key().clone();
            let connection = entry.value();

            if let Ok(last_heartbeat) = connection.last_heartbeat.lock() {
                clients.push(OnlineClientInfo {
                    client_id,
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

    async fn run(&self, host: String, port: u16) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;
        info!("🚀 v-connect-im WebSocket Server starting on {}", addr);
        info!("📡 Waiting for connections...");

        while let Ok((stream, peer_addr)) = listener.accept().await {
            let connections = self.connections.clone();
            let server = self.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_connection(stream, peer_addr, connections, server).await
                {
                    error!("Connection error from {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }

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
                if let Err(e) = ws_sender.send(msg).await {
                    error!("Failed to send message to {}: {}", client_id_clone, e);
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

        info!("✅ Client {} connected from {}", client_id, peer_addr);

        // 发送客户端上线Webhook事件 / Send client online webhook event
        service::webhook::send_client_online_webhook(&server, &client_id, &None, &peer_addr).await;
        let location = v::comm::geo::get_region_by_ip(None).await?;

        // 发送欢迎消息
        let welcome_msg = ConnectResponse {
            status: "connected".to_string(),
            message: format!("Welcome to v-connect-im Server, location: {:?}", location),
            client_id: client_id.clone(),
        };

        server
            .send_message_to_client(
                &client_id,
                Message::Text(serde_json::to_string(&welcome_msg)?),
            )
            .await?;

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
        }

        Ok(())
    }

    // 向指定客户端发送消息
    async fn send_message_to_client(&self, client_id: &str, message: Message) -> Result<()> {
        if let Some(connection) = self.connections.get(client_id) {
            connection
                .sender
                .send(message)
                .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
            debug!("📤 Sent message to client {}", client_id);
            Ok(())
        } else {
            warn!("⚠️  Client {} not found for message delivery", client_id);
            Err(anyhow::anyhow!("Client {} not found", client_id))
        }
    }

    // 发送关闭消息给客户端 / Send close message to client
    async fn send_close_message(&self, client_id: &str) -> Result<()> {
        if let Some(connection) = self.connections.get(client_id) {
            // 发送WebSocket关闭帧 / Send WebSocket close frame
            connection
                .sender
                .send(Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: std::borrow::Cow::Borrowed("Connection timeout"),
                })))
                .map_err(|e| anyhow::anyhow!("Failed to send close message: {}", e))?;
            debug!("🔒 Sent close message to client {}", client_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Client {} not found for close message",
                client_id
            ))
        }
    }

    // 广播消息给所有客户端
    async fn broadcast_message(&self, message: Message) -> Result<()> {
        let message_str = match &message {
            Message::Text(text) => text.clone(),
            _ => return Ok(()),
        };

        let mut disconnected_clients = Vec::new();

        for entry in self.connections.iter() {
            let client_id = entry.key().clone();
            let connection = entry.value();

            if let Err(_) = connection.sender.send(Message::Text(message_str.clone())) {
                disconnected_clients.push(client_id);
            }
        }

        // 移除断开的客户端
        for client_id in disconnected_clients {
            self.connections.remove(&client_id);
        }

        Ok(())
    }

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
                match serde_json::from_str::<WuKongMessage>(&text) {
                    Ok(wk_msg) => {
                        match wk_msg.msg_type.as_str() {
                            "ping" => {
                                debug!("🏓 Ping from {}", client_id);
                                // 更新心跳时间 / Update heartbeat time
                                self.update_heartbeat(client_id).await;

                                let pong_msg = WuKongMessage {
                                    msg_type: "pong".to_string(),
                                    data: serde_json::json!({
                                        "timestamp": chrono::Utc::now().timestamp_millis(),
                                        "client_id": client_id
                                    }),
                                    target_id: None,
                                };
                                let pong_json = serde_json::to_string(&pong_msg)?;
                                self.send_message_to_client(client_id, Message::Text(pong_json))
                                    .await?;
                            }
                            "online_clients" => {
                                info!("📋 Online clients query from {}", client_id);
                                let online_clients = self.get_online_clients().await;
                                let response_msg = WuKongMessage {
                                    msg_type: "online_clients_response".to_string(),
                                    data: serde_json::json!(online_clients),
                                    target_id: None,
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
                                let auth_response = WuKongMessage {
                                    msg_type: "auth_response".to_string(),
                                    data: serde_json::json!({
                                        "status": "success",
                                        "message": "Authentication successful"
                                    }),
                                    target_id: None,
                                };
                                let auth_json = serde_json::to_string(&auth_response)?;
                                self.send_message_to_client(client_id, Message::Text(auth_json))
                                    .await?;
                            }
                            "message" => {
                                info!("💬 Message from {}: {:?}", client_id, wk_msg.data);

                                // 如果有目标ID，发送给指定客户端，否则回声
                                if let Some(target_id) = &wk_msg.target_id {
                                    if target_id != client_id {
                                        let forward_msg = WuKongMessage {
                                            msg_type: "forwarded_message".to_string(),
                                            data: serde_json::json!({
                                                "from": client_id,
                                                "content": wk_msg.data,
                                                "timestamp": chrono::Utc::now().timestamp_millis()
                                            }),
                                            target_id: None,
                                        };
                                        let forward_json = serde_json::to_string(&forward_msg)?;
                                        let delivery_result = self
                                            .send_message_to_client(
                                                target_id,
                                                Message::Text(forward_json),
                                            )
                                            .await;

                                        match delivery_result {
                                            Ok(_) => {
                                                // 发送消息送达Webhook事件 / Send message delivered webhook event
                                                let message_id = Uuid::new_v4().to_string();
                                                service::webhook::send_message_webhook(
                                                    self,
                                                    &message_id,
                                                    client_id,
                                                    &None, // from_uid
                                                    &Some(target_id.clone()),
                                                    &None, // to_uid
                                                    &wk_msg.data,
                                                    "message",
                                                    "delivered",
                                                    Some(chrono::Utc::now().timestamp_millis()),
                                                )
                                                .await;

                                                // 同时给发送者确认
                                                let confirm_msg = WuKongMessage {
                                                    msg_type: "message_sent".to_string(),
                                                    data: serde_json::json!({
                                                        "to": target_id,
                                                        "status": "delivered"
                                                    }),
                                                    target_id: None,
                                                };
                                                let confirm_json =
                                                    serde_json::to_string(&confirm_msg)?;
                                                self.send_message_to_client(
                                                    client_id,
                                                    Message::Text(confirm_json),
                                                )
                                                .await?;
                                            }
                                            Err(e) => {
                                                // 发送消息失败Webhook事件 / Send message failed webhook event
                                                let message_id = Uuid::new_v4().to_string();
                                                service::webhook::send_message_webhook(
                                                    self,
                                                    &message_id,
                                                    client_id,
                                                    &None, // from_uid
                                                    &Some(target_id.clone()),
                                                    &None, // to_uid
                                                    &wk_msg.data,
                                                    "message",
                                                    "failed",
                                                    None,
                                                )
                                                .await;
                                                return Err(e);
                                            }
                                        }
                                    }
                                } else {
                                    // 回声给发送者
                                    let echo_msg = WuKongMessage {
                                        msg_type: "message_echo".to_string(),
                                        data: serde_json::json!({
                                            "original": wk_msg.data,
                                            "from": client_id,
                                            "timestamp": chrono::Utc::now().timestamp_millis()
                                        }),
                                        target_id: None,
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
                                if let Some(target_id) = &wk_msg.target_id {
                                    let private_msg = WuKongMessage {
                                        msg_type: "private_message".to_string(),
                                        data: serde_json::json!({
                                            "from": client_id,
                                            "content": wk_msg.data,
                                            "timestamp": chrono::Utc::now().timestamp_millis()
                                        }),
                                        target_id: None,
                                    };
                                    let private_json = serde_json::to_string(&private_msg)?;
                                    let delivery_result = self
                                        .send_message_to_client(
                                            target_id,
                                            Message::Text(private_json),
                                        )
                                        .await;

                                    match delivery_result {
                                        Ok(_) => {
                                            // 发送私聊消息送达Webhook事件 / Send private message delivered webhook event
                                            let message_id = Uuid::new_v4().to_string();
                                            service::webhook::send_message_webhook(
                                                self,
                                                &message_id,
                                                client_id,
                                                &None, // from_uid
                                                &Some(target_id.clone()),
                                                &None, // to_uid
                                                &wk_msg.data,
                                                "private_message",
                                                "delivered",
                                                Some(chrono::Utc::now().timestamp_millis()),
                                            )
                                            .await;
                                        }
                                        Err(e) => {
                                            // 发送私聊消息失败Webhook事件 / Send private message failed webhook event
                                            let message_id = Uuid::new_v4().to_string();
                                            service::webhook::send_message_webhook(
                                                self,
                                                &message_id,
                                                client_id,
                                                &None, // from_uid
                                                &Some(target_id.clone()),
                                                &None, // to_uid
                                                &wk_msg.data,
                                                "private_message",
                                                "failed",
                                                None,
                                            )
                                            .await;
                                            return Err(e);
                                        }
                                    }
                                } else {
                                    let error_msg = WuKongMessage {
                                        msg_type: "error".to_string(),
                                        data: serde_json::json!({
                                            "message": "private_message requires target_id"
                                        }),
                                        target_id: None,
                                    };
                                    let error_json = serde_json::to_string(&error_msg)?;
                                    self.send_message_to_client(
                                        client_id,
                                        Message::Text(error_json),
                                    )
                                    .await?;
                                }
                            }
                            _ => {
                                warn!(
                                    "⚠️  Unknown message type from {}: {}",
                                    client_id, wk_msg.msg_type
                                );
                                let error_msg = WuKongMessage {
                                    msg_type: "error".to_string(),
                                    data: serde_json::json!({
                                        "message": format!("Unknown message type: {}", wk_msg.msg_type)
                                    }),
                                    target_id: None,
                                };
                                let error_json = serde_json::to_string(&error_msg)?;
                                self.send_message_to_client(client_id, Message::Text(error_json))
                                    .await?;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️  Invalid JSON from {}: {}", client_id, e);
                        let error_msg = WuKongMessage {
                            msg_type: "error".to_string(),
                            data: serde_json::json!({
                                "message": "Invalid JSON format"
                            }),
                            target_id: None,
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
        webhook_config: WebhookConfig,
        event: WebhookEvent,
    ) -> Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(webhook_config.timeout_ms))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let mut request = client.post(&webhook_config.url).json(&event);

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

// 实现Clone trait用于多线程共享
impl Clone for VConnectIMServer {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            webhook_config: self.webhook_config.clone(),
        }
    }
}

/// 启动HTTP服务器 / Start HTTP server
async fn start_http_server(server: Arc<VConnectIMServer>, host: String, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    // 启动前打印路由映射（自动生成） / Print auto-generated route map before start
    api_registry::print_routes(&addr, &["Logger"]);

    // 使用 actix-web 构建路由（自动注册） / Build routes with actix-web (auto registry)
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .configure(api_registry::configure)
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

    let webhook_url: Option<String> = cm.get::<String>("webhook.url").ok();
    let webhook_timeout_ms: u64 = cm.get_or("webhook.timeout_ms", 3000000_i64) as u64;
    let webhook_secret: Option<String> = cm.get::<String>("webhook.secret").ok();
    let webhook_enabled: bool = cm.get_or("webhook.enabled", false);

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
    let server = if webhook_enabled && webhook_url.is_some() {
        let webhook_config = WebhookConfig {
            url: webhook_url.unwrap(),
            timeout_ms: webhook_timeout_ms,
            secret: webhook_secret,
            enabled: true,
        };
        Arc::new(VConnectIMServer::new().with_webhook_config(webhook_config))
    } else {
        Arc::new(VConnectIMServer::new())
    };

    let server_clone = server.clone();
    let server_http = server.clone();

    // 启动自动心跳清理任务 / Start automatic heartbeat cleanup task
    tokio::spawn(async move {
        // 根据超时时间动态调整清理间隔 / Adjust cleanup interval based on timeout
        let cleanup_interval_ms = if timeout_ms <= 1000 {
            timeout_ms / 2 // 对于短超时，每半周期清理一次 / For short timeouts, cleanup every half cycle
        } else if timeout_ms <= 10000 {
            1000 // 对于中等超时，每秒清理一次 / For medium timeouts, cleanup every second
        } else {
            5000 // 对于长超时，每5秒清理一次 / For long timeouts, cleanup every 5 seconds
        };

        info!(
            "⏰ Cleanup interval set to {}ms for timeout {}ms",
            cleanup_interval_ms, timeout_ms
        );
        let mut cleanup_interval = interval(Duration::from_millis(cleanup_interval_ms));
        loop {
            cleanup_interval.tick().await;
            server_clone.cleanup_timeout_connections(timeout_ms).await; // 使用配置的毫秒超时 / Use configured millisecond timeout
        }
    });

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

    info!("✅ Server shutdown successfully");

    Ok(())
}
