use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(handle)));
}

async fn handle(_server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let built = include_str!(concat!(env!("OUT_DIR"), "/openapi.json"));
    let val: serde_json::Value = serde_json::from_str(built).unwrap_or_else(|_| serde_json::json!({"openapi":"3.0.3","info":{"title":"v-connect-im","version":"v1"},"paths":{}}));
    respond_any(StatusCode::OK, val)
}
