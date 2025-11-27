use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct StatusQuery {
    pub uid: String,
    pub message_id: String,
}

#[derive(serde::Serialize, Debug)]
pub struct StatusResponse {
    pub uid: String,
    pub message_id: String,
    pub acked: bool,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(message_status_handle)));
}

pub async fn message_status_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<StatusQuery>,
) -> impl Responder {
    let acked = server
        .acked_ids
        .get(&query.uid)
        .map(|set| set.contains(&query.message_id))
        .unwrap_or(false);
    respond_any(
        StatusCode::OK,
        StatusResponse { uid: query.uid.clone(), message_id: query.message_id.clone(), acked },
    )
}
