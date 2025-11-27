use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

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
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<CleanupRequest>,
) -> impl Responder {
    let lim = req.limit.unwrap_or(1000);
    match server.storage.cleanup_offline(&req.uid, req.before_ts, lim) {
        Ok(removed) => respond_any(StatusCode::OK, serde_json::json!({"removed": removed})),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({"error": format!("{}", e)})),
    }
}

