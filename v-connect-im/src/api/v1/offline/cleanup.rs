use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct CleanupRequest {
    pub uid: String,
    pub before_ts: i64,
    pub limit: Option<usize>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(offline_cleanup_handle)));
}

pub async fn offline_cleanup_handle(
    _server: web::Data<Arc<VConnectIMServer>>,
    _req: web::Json<CleanupRequest>,
) -> impl Responder {
    // 离线消息清理功能已迁移到插件 / Offline cleanup feature migrated to plugin
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Feature migrated to plugin",
            "message": "离线消息清理需要存储插件 / Offline cleanup requires storage plugin"
        }),
    )
}
