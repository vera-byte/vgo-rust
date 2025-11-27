use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use tracing::warn;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct CreateRequest {
    pub room_id: Option<String>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(room_create_handle)));
}

pub async fn room_create_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<CreateRequest>,
) -> impl Responder {
    let rid = req
        .room_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    server.rooms.entry(rid.clone()).or_default();
    let event_payload = serde_json::json!({ "room_id": rid.clone() });
    if let Err(e) = server
        .plugin_registry
        .emit_custom("room.created", &event_payload)
        .await
    {
        warn!("room.created plugin event failed: {}", e);
    }
    // 创建房间不需要持久化成员，但可确保空集合存在 / Room creation ensures empty in-memory set
    respond_any(
        StatusCode::OK,
        serde_json::json!({
            "room_id": rid
        }),
    )
}
