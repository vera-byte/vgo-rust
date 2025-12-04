use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct AckRequest {
    pub uid: String,
    pub message_ids: Vec<String>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(offline_ack_handle)));
}

pub async fn offline_ack_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<AckRequest>,
) -> impl Responder {
    // 通过存储插件确认离线消息 / Acknowledge offline messages through storage plugin
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool.storage_ack_offline(&req.uid, &req.message_ids).await {
            Ok(removed) => respond_any(
                StatusCode::OK,
                serde_json::json!({
                    "removed": removed
                }),
            ),
            Err(e) => respond_any(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": format!("存储插件错误 / Storage plugin error: {}", e)
                }),
            ),
        }
    } else {
        respond_any(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({
                "error": "存储插件未初始化 / Storage plugin not initialized"
            }),
        )
    }
}
