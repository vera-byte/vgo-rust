use anyhow::Result;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, warn};

use crate::server::VConnectIMServer;

/// 向指定客户端发送消息 / Send message to specific client
impl VConnectIMServer {
    pub async fn send_message_to_client(&self, client_id: &str, message: Message) -> Result<()> {
        if let Some(connection) = self.connections.get(client_id) {
            connection.sender.send(message).map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
            debug!("📤 Sent message to client {}", client_id);
            Ok(())
        } else {
            warn!("⚠️  Client {} not found for message delivery", client_id);
            Err(anyhow::anyhow!("Client {} not found", client_id))
        }
    }

    /// 发送关闭消息 / Send close message
    pub async fn send_close_message(&self, client_id: &str) -> Result<()> {
        if let Some(connection) = self.connections.get(client_id) {
            connection.sender.send(Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                reason: std::borrow::Cow::Borrowed("Connection timeout"),
            }))).map_err(|e| anyhow::anyhow!("Failed to send close message: {}", e))?;
            debug!("🔒 Sent close message to client {}", client_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client {} not found for close message", client_id))
        }
    }

    /// 广播文本消息 / Broadcast text message
    pub async fn broadcast_message(&self, message: Message) -> Result<()> {
        let message_str = match &message { Message::Text(text) => text.clone(), _ => return Ok(()) };
        let mut disconnected_clients = Vec::new();
        for entry in self.connections.iter() {
            let client_id = entry.key().clone();
            let connection = entry.value();
            if connection.sender.send(Message::Text(message_str.clone())).is_err() { disconnected_clients.push(client_id); }
        }
        for client_id in disconnected_clients { self.connections.remove(&client_id); }
        Ok(())
    }
}

