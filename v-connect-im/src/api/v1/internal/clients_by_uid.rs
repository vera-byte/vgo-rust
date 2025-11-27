use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct ClientsByUidQuery { pub uid: String }

#[derive(serde::Serialize, Debug)]
pub struct ClientsByUidResponse { pub client_ids: Vec<String> }

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(handle)));
}

pub async fn handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<ClientsByUidQuery>,
) -> impl Responder {
    let ids: Vec<String> = server
        .uid_clients
        .get(&query.uid)
        .map(|set| set.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    respond_any(StatusCode::OK, ClientsByUidResponse { client_ids: ids })
}
