use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct HasClientQuery {
    pub client_id: String,
}

// 路由注册入口（GET）/ Register route (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(has_client_handle)));
}

// 检查本节点是否持有指定客户端连接 / Check if local node holds the client connection
pub async fn has_client_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<HasClientQuery>,
) -> impl Responder {
    let exists = server.connections.contains_key(&query.client_id);
    respond_any(StatusCode::OK, serde_json::json!({ "exists": exists }))
}

