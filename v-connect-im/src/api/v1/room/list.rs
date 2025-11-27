use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Serialize, Debug)]
pub struct RoomListResponse {
    pub rooms: Vec<String>,
    pub count: usize,
}

#[derive(serde::Deserialize)]
pub struct RoomListQuery {
    pub prefix: Option<String>,
    pub limit: Option<usize>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(room_list_handle)));
}

pub async fn room_list_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<RoomListQuery>,
) -> impl Responder {
    match server
        .storage
        .list_rooms_by_prefix(query.prefix.as_deref(), query.limit)
    {
        Ok(rooms) => respond_any(StatusCode::OK, RoomListResponse { count: rooms.len(), rooms }),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({
            "error": format!("{}", e)
        })),
    }
}
