use anyhow::Result;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

use crate::server::{Connection, VConnectIMServer};
use crate::domain::message::ImMessage;

/// 处理基础进入消息（ping/online）/ Handle basic incoming messages (ping/online)
pub async fn dispatch_basic(
    server: &VConnectIMServer,
    message: Message,
    client_id: &str,
    _connections: &std::sync::Arc<dashmap::DashMap<String, Connection>>,
) -> Result<()> {
    match message {
        Message::Text(text) => {
            debug!("📨 Received text from {}: {}", client_id, text);
            match serde_json::from_str::<ImMessage>(&text) {
                Ok(wk_msg) => match wk_msg.msg_type.as_str() {
                    "ping" => {
                        debug!("🏓 Ping from {}", client_id);
                        server.update_heartbeat(client_id).await;
                        let pong_msg = ImMessage { msg_type: "pong".to_string(), data: serde_json::json!({"timestamp": chrono::Utc::now().timestamp_millis(), "client_id": client_id}), target_uid: None };
                        let pong_json = serde_json::to_string(&pong_msg)?;
                        server.send_message_to_client(client_id, Message::Text(pong_json)).await?;
                    }
                    "online_clients" => {
                        info!("📋 Online clients query from {}", client_id);
                        let online_clients = server.get_online_clients().await;
                        let response_msg = ImMessage { msg_type: "online_clients_response".to_string(), data: serde_json::json!(online_clients), target_uid: None };
                        let response_json = serde_json::to_string(&response_msg)?;
                        server.send_message_to_client(client_id, Message::Text(response_json)).await?;
                    }
                    _ => {}
                },
                Err(_) => {
                    let err_msg = ImMessage { msg_type: "error".to_string(), data: serde_json::json!({"message":"invalid json"}), target_uid: None};
                    let err_json = serde_json::to_string(&err_msg)?;
                    server.send_message_to_client(client_id, Message::Text(err_json)).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

