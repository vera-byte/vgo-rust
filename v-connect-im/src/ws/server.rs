use anyhow::Result;
use tokio::net::TcpListener;
use std::sync::Arc;
use tracing::info;

use crate::server::VConnectIMServer;

/// 启动WS监听 / Start WS listener
impl VConnectIMServer {
    pub async fn run(&self, host: String, port: u16) -> Result<()> {
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await?;
        info!("🚀 v-connect-im WebSocket Server starting on {}", addr);
        info!("📡 Waiting for connections...");

        if let Ok(cm) = v::get_global_config_manager() {
            let enabled = cm.get_or("quic.enabled", 0_i64) == 1;
            let quic_port = cm.get_or("quic.port", port as i64) as u16;
            let quic_host: String = cm.get_or("quic.host", host.clone());
            if enabled {
                let quic_addr = format!("{}:{}", quic_host, quic_port).parse().unwrap();
                let quic = crate::net::quic::QuicServer::new(Arc::new(self.clone()), quic_addr);
                std::mem::drop(quic.start().await);
                info!("🟢 QUIC listening at {}", quic_addr);
            }
        }

        while let Ok((stream, peer_addr)) = listener.accept().await {
            let connections = self.connections.clone();
            let server = self.clone();

            tokio::spawn(async move {
                if let Err(e) = crate::ws::connection::handle_connection(stream, peer_addr, connections, server).await {
                    tracing::error!("Connection error from {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}

