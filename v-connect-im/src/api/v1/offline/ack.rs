use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

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
    match server.storage.ack_offline(&req.uid, &req.message_ids) {
        Ok(removed) => respond_any(StatusCode::OK, serde_json::json!({
            "removed": removed
        })),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({
            "error": format!("{}", e)
        })),
    }
}

