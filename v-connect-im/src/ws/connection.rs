use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

use crate::server::{Connection, VConnectIMServer};

/// 处理新连接 / Handle new connection
pub async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    connections: Arc<dashmap::DashMap<String, Connection>>,
    server: VConnectIMServer,
) -> Result<()> {
    tracing::info!("📨 New connection from: {}", peer_addr);

    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let client_id = Uuid::new_v4().to_string();

    let client_id_clone = client_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let is_close = matches!(&msg, Message::Close(_));
            if let Err(e) = ws_sender.send(msg).await {
                tracing::error!("Failed to send message to {}: {}", client_id_clone, e);
                break;
            }
            if is_close {
                let _ = ws_sender.close().await;
                break;
            }
        }
    });

    let connection = Connection { client_id: client_id.clone(), uid: None, addr: peer_addr, sender: tx, last_heartbeat: Arc::new(std::sync::Mutex::new(std::time::Instant::now())) };
    connections.insert(client_id.clone(), connection);
    server.directory.register_client_location(&client_id, &server.node_id);
    tracing::info!("✅ Client {} connected from {}", client_id, peer_addr);

    crate::service::webhook::send_client_online_webhook(&server, &client_id, &None, &peer_addr).await;
    let welcome_text = "Welcome to v-connect-im Server".to_string();
    let welcome_msg = crate::domain::message::ConnectResponse { status: "connected".to_string(), message: welcome_text };
    server.send_message_to_client(&client_id, Message::Text(serde_json::to_string(&welcome_msg)?)).await?;

    let auth_deadline_ms: u64 = v::get_global_config_manager().ok().map(|cm| cm.get_or("auth.deadline_ms", 1000_u64)).unwrap_or(1000);
    {
        let watchdog_client = client_id.clone();
        let watchdog_connections = connections.clone();
        let watchdog_server = server.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(auth_deadline_ms)).await;
            if let Some(conn) = watchdog_connections.get(&watchdog_client) {
                if conn.uid.is_none() {
                    let _ = watchdog_server.send_close_message(&watchdog_client).await;
                    watchdog_connections.remove(&watchdog_client);
                    tracing::warn!("disconnecting unauthenticated client_id={}", watchdog_client);
                }
            }
        });
    }

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(message) => {
                if let Err(e) = server.handle_incoming_message(message, &client_id, &connections).await { tracing::error!("Error handling message from {}: {}", client_id, e); }
            }
            Err(e) => {
                tracing::error!("WebSocket error from {}: {}", client_id, e);
                break;
            }
        }
    }

    let connection_info = connections.remove(&client_id);
    send_task.abort();
    tracing::info!("👋 Client {} disconnected", client_id);
    if let Some((_, connection)) = connection_info {
        let connected_at = chrono::Utc::now().timestamp_millis() - connection.last_heartbeat.lock().unwrap().elapsed().as_millis() as i64;
        crate::service::webhook::send_client_offline_webhook(&server, &client_id, &connection.uid, &connection.addr, connected_at).await;
        if let Some(uid) = &connection.uid { if let Some(set) = server.uid_clients.get_mut(uid) { set.remove(&client_id); } }
    }
    Ok(())
}
