use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct RateLimitRequest { pub uid: String, pub limit_per_sec: usize }

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(uid_rate_limit_handle)));
}

pub async fn uid_rate_limit_handle(server: web::Data<Arc<VConnectIMServer>>, req: web::Json<RateLimitRequest>) -> impl Responder {
    if req.limit_per_sec == 0 { server.uid_rate_limits.remove(&req.uid); } else { server.uid_rate_limits.insert(req.uid.clone(), (req.limit_per_sec, 0, chrono::Utc::now().timestamp_millis())); }
    respond_any(StatusCode::OK, serde_json::json!({"uid": req.uid, "limit_per_sec": req.limit_per_sec}))
}

