use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct BlockRequest { pub uid: String, pub block: bool }

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(uid_block_handle)));
}

pub async fn uid_block_handle(server: web::Data<Arc<VConnectIMServer>>, req: web::Json<BlockRequest>) -> impl Responder {
    if req.block { server.blocked_uids.insert(req.uid.clone()); } else { server.blocked_uids.remove(&req.uid); }
    respond_any(StatusCode::OK, serde_json::json!({"uid": req.uid, "blocked": req.block}))
}

