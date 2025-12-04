use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct UnreadQuery {
    pub uid: String,
    pub room_id: String,
}

#[derive(serde::Serialize, Debug)]
pub struct UnreadResponse {
    pub uid: String,
    pub room_id: String,
    pub count: usize,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(room_unread_handle)));
}

pub async fn room_unread_handle(
    _server: web::Data<Arc<VConnectIMServer>>,
    _query: web::Query<UnreadQuery>,
) -> impl Responder {
    // 房间未读数功能已迁移到插件 / Room unread count feature migrated to plugin
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Feature migrated to plugin",
            "message": "房间未读数需要存储插件 / Room unread count requires storage plugin"
        }),
    )
}
