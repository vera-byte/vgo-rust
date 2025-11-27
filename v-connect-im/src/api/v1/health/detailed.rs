use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use v::response::respond_any;
use v::HealthCheck;
use std::sync::Arc;
use crate::VConnectIMServer;

pub const ROUTE_PATH: &str = "/health/detailed";

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(health_detailed_handle)));
}

// 详细健康检查
// Detailed health check
pub async fn health_detailed_handle(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let status = server.check_health().await;
    let online = server.get_online_clients().await;
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
        let payload = serde_json::json!({
            "status": status,
            "service": "v-connect-im",
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "details": {
                "online_clients": online.total_count,
                "websocket_port": 5200,
                "http_port": 8080,
                "uptime_seconds": uptime,
                "version": "0.1.0",
                "quic_conn_count": server.quic_conn_count.load(std::sync::atomic::Ordering::Relaxed),
                "quic_path_updates": server.quic_path_updates.load(std::sync::atomic::Ordering::Relaxed),
                "quic_stream_sent": server.quic_stream_sent.load(std::sync::atomic::Ordering::Relaxed),
                "quic_dgram_sent": server.quic_dgram_sent.load(std::sync::atomic::Ordering::Relaxed)
                ,"quic_stream_recv": server.quic_stream_recv.load(std::sync::atomic::Ordering::Relaxed)
                ,"quic_dgram_recv": server.quic_dgram_recv.load(std::sync::atomic::Ordering::Relaxed)
                ,"blocked_uids_count": server.blocked_uids.len()
                ,"rate_limits_count": server.uid_rate_limits.len()
            }
        });
    respond_any(StatusCode::OK, payload)
}
