use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(connection_list_handle)));
}

// 获取 WebSocket 连接列表
// Get WebSocket connection list
pub async fn connection_list_handle(server: web::Data<Arc<VConnectIMServer>>) -> impl Responder {
    let online = server.get_online_clients().await; // 在线列表 / online list
    respond_any(StatusCode::OK, online)
}
