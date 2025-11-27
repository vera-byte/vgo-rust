use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::{VConnectIMServer, HttpBroadcastResponse};

#[derive(serde::Deserialize)]
pub struct GroupSendRequest {
    pub room_id: String,
    pub from_client_id: String,
    pub content: serde_json::Value,
    pub message_type: Option<String>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(group_send_handle)));
}

pub async fn group_send_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    req: web::Json<GroupSendRequest>,
) -> impl Responder {
    let resp: HttpBroadcastResponse = server
        .http_group_send_message(
            req.room_id.clone(),
            req.from_client_id.clone(),
            req.content.clone(),
            req.message_type.clone(),
        )
        .await;
    let code = if resp.success { StatusCode::OK } else { StatusCode::BAD_REQUEST };
    respond_any(code, resp)
}

