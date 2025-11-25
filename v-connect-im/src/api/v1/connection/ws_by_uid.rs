use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

pub const ROUTE_PATH: &str = "/v1/connection/ws_by_uid";

// 路由注册入口（GET）
// Route registration entry (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(ws_by_uid_handle)));
}

#[derive(serde::Deserialize)]
struct QueryUid {
    uid: String,
}

// 通过用户ID获取 WebSocket 连接地址
// Get WebSocket connection URLs by user ID
pub async fn ws_by_uid_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<QueryUid>,
) -> impl Responder {
    let cm = match v::get_global_config_manager() {
        Ok(c) => c,
        Err(e) => return respond_any(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)),
    };
    // 使用配置的 host 与 ws_port 构造 ws URL / build ws url from config
    let host: String = cm.get_or("server.host", "127.0.0.1".to_string());
    let ws_port: u16 = cm.get_or("server.ws_port", 5200_i64) as u16;
    let base = format!("ws://{}:{}", host, ws_port);

    let mut urls = Vec::new();
    for entry in server.connections.iter() {
        let conn = entry.value();
        if conn.uid.as_deref() == Some(&query.uid) {
            urls.push(serde_json::json!({
                "client_id": entry.key().clone(),
                "ws_url": base,
            }));
        }
    }

    let payload = serde_json::json!({
        "uid": query.uid,
        "connections": urls,
    });
    respond_any(StatusCode::OK, payload)
}
