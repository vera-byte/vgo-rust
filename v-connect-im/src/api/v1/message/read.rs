use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::{VConnectIMServer};

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
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<ReadRequest>,
) -> impl Responder {
    let rr = crate::storage::ReadReceipt {
        message_id: req.message_id.clone(),
        uid: req.uid.clone(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    match server.storage.record_read(&rr) {
        Ok(_) => respond_any(StatusCode::OK, serde_json::json!({"ok": true})),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({"error": format!("{}", e)})),
    }
}

pub async fn read_list_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<ReadListQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(100);
    match server.storage.list_reads(&query.uid, limit) {
        Ok(list) => respond_any(StatusCode::OK, list),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({"error": format!("{}", e)})),
    }
}

