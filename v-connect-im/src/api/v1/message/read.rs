use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct ReadRequest {
    pub uid: String,
    pub message_id: String,
}

#[derive(serde::Deserialize)]
pub struct ReadListQuery {
    pub uid: String,
    pub limit: Option<usize>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(read_mark_handle)));
    cfg.service(web::resource(format!("{}/list", path)).route(web::get().to(read_list_handle)));
}

pub async fn read_mark_handle(
    _server: web::Data<Arc<VConnectIMServer>>,
    _req: web::Json<ReadRequest>,
) -> impl Responder {
    // 已读回执功能已迁移到插件 / Read receipt feature migrated to plugin
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Feature migrated to plugin",
            "message": "已读回执功能需要存储插件 / Read receipt requires storage plugin"
        }),
    )
}

pub async fn read_list_handle(
    _server: web::Data<Arc<VConnectIMServer>>,
    _query: web::Query<ReadListQuery>,
) -> impl Responder {
    // 已读回执列表功能已迁移到插件 / Read receipt list feature migrated to plugin
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Feature migrated to plugin",
            "message": "已读回执列表需要存储插件 / Read receipt list requires storage plugin"
        }),
    )
}
