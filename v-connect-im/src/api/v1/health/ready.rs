use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use v::response::respond_any;
use std::sync::Arc;
use crate::VConnectIMServer;

pub const ROUTE_PATH: &str = "/health/ready";

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(health_ready_handle)));
}

// 就绪检查
// Readiness check
pub async fn health_ready_handle(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let online = server.get_online_clients().await;
    let is_ready = online.total_count < 10_000;
    let payload = serde_json::json!({
        "ready": is_ready,
        "service": "v-connect-im",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "online_clients": online.total_count,
        "max_supported": 10000
    });
    let code = if is_ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    respond_any(code, payload)
}
