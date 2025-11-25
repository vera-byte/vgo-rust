use actix_web::Responder;
use actix_web::http::StatusCode;
use v::response::respond_any;

pub const ROUTE_PATH: &str = "/health/live";

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(actix_web::web::resource(path).route(actix_web::web::get().to(health_live_handle)));
}

// 存活检查
// Liveness check
pub async fn health_live_handle() -> impl Responder {
    let payload = serde_json::json!({
        "alive": true,
        "service": "v-connect-im",
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    respond_any(StatusCode::OK, payload)
}
