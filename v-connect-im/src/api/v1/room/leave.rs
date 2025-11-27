use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use tracing::warn;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct LeaveRequest {
    pub uid: String,
    pub room_id: String,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(room_leave_handle)));
}

pub async fn room_leave_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<LeaveRequest>,
) -> impl Responder {
    if let Some(set) = server.rooms.get_mut(&req.room_id) {
        set.remove(&req.uid);
    }
    // 持久化移除 / Persist remove
    let _ = server.storage.remove_room_member(&req.room_id, &req.uid);
    let event_payload = serde_json::json!({
        "room_id": req.room_id.clone(),
        "uid": req.uid.clone()
    });
    if let Err(e) = server
        .plugin_registry
        .emit_custom("room.left", &event_payload)
        .await
    {
        warn!("room.left plugin event failed: {}", e);
    }
    respond_any(
        StatusCode::OK,
        serde_json::json!({
            "room_id": req.room_id,
            "uid": req.uid
        }),
    )
}
