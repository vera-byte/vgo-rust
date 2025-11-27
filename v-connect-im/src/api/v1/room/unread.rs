use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

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
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<UnreadQuery>,
) -> impl Responder {
    match server.storage.offline_count_by_room(&query.uid, &query.room_id) {
        Ok(count) => respond_any(StatusCode::OK, UnreadResponse { uid: query.uid.clone(), room_id: query.room_id.clone(), count }),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({"error": format!("{}", e)})),
    }
}

