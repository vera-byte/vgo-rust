use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use tracing::warn;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct JoinRequest {
    pub uid: String,
    pub room_id: String,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(room_join_handle)));
}

pub async fn room_join_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<JoinRequest>,
) -> impl Responder {
    // 内存加入 / In-memory add
    server
        .rooms
        .entry(req.room_id.clone())
        .or_default()
        .insert(req.uid.clone());

    // 通过存储插件持久化 / Persist through storage plugin
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        if let Err(e) = pool.storage_add_room_member(&req.room_id, &req.uid).await {
            warn!(
                "存储插件添加房间成员失败 / Storage plugin add room member failed: {}",
                e
            );
        }
    }
    let event_payload = serde_json::json!({
        "room_id": req.room_id.clone(),
        "uid": req.uid.clone()
    });
    if let Err(e) = server
        .plugin_registry
        .emit_custom("room.joined", &event_payload)
        .await
    {
        warn!("room.joined plugin event failed: {}", e);
    }
    respond_any(
        StatusCode::OK,
        serde_json::json!({
            "room_id": req.room_id,
            "uid": req.uid
        }),
    )
}
