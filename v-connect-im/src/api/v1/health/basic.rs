use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use v::response::respond_any;
use v::HealthCheck;
use std::sync::Arc;
use crate::VConnectIMServer;

pub const ROUTE_PATH: &str = "/health";

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(health_basic_handle)));
}

// 基础健康检查
// Basic health check
pub async fn health_basic_handle(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let status = server.check_health().await;
    respond_any(StatusCode::OK, status)
}
