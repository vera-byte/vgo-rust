use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct MembersQuery {
    pub room_id: String,
}

#[derive(serde::Serialize, Debug)]
pub struct MembersResponse {
    pub room_id: String,
    pub members: Vec<String>,
    pub count: usize,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(room_members_handle)));
}

pub async fn room_members_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<MembersQuery>,
) -> impl Responder {
    match server.storage.list_room_members(&query.room_id) {
        Ok(members) => respond_any(
            StatusCode::OK,
            MembersResponse { room_id: query.room_id.clone(), count: members.len(), members },
        ),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({
            "error": format!("{}", e)
        })),
    }
}

