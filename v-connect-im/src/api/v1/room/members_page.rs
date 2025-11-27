use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct MembersPageQuery {
    pub room_id: String,
    pub uid_prefix: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(serde::Serialize, Debug)]
pub struct MembersPageResponse {
    pub room_id: String,
    pub members: Vec<String>,
    pub next_cursor: Option<String>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(members_page_handle)));
}

pub async fn members_page_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<MembersPageQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50);
    match server.storage.list_room_members_paginated(
        &query.room_id,
        query.uid_prefix.as_deref(),
        query.cursor.clone(),
        limit,
    ) {
        Ok((members, next)) => respond_any(StatusCode::OK, MembersPageResponse { room_id: query.room_id.clone(), members, next_cursor: next }),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({"error": format!("{}", e)})),
    }
}

