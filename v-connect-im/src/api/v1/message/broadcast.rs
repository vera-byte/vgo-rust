use crate::{HttpBroadcastRequest, VConnectIMServer};
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

// 路由注册入口（POST）
// Route registration entry (POST)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(message_broadcast_handle)));
}

// 广播消息接口
// Broadcast message API
pub async fn message_broadcast_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    request: web::Json<HttpBroadcastRequest>,
) -> impl Responder {
    let resp = server.http_broadcast_message(request.into_inner()).await;
    let code = if resp.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    respond_any(code, resp)
}
