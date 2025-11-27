use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use tokio_tungstenite::tungstenite::Message;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct ForwardClientRequest {
    pub client_id: String,
    pub text: String,
}

// 路由注册入口（POST）/ Register route (POST)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(forward_client_handle)));
}

// 向本节点指定client_id转发文本消息 / Forward text to local client by client_id
pub async fn forward_client_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<ForwardClientRequest>,
) -> impl Responder {
    match server.send_message_to_client(&req.client_id, Message::Text(req.text.clone())).await {
        Ok(_) => respond_any(StatusCode::OK, serde_json::json!({ "ok": true })),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({ "error": format!("{}", e) })),
    }
}

