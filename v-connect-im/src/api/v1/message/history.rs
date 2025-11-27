use actix_web::{web, Responder};
use actix_web::http::StatusCode;
use std::sync::Arc;
use v::response::respond_any;
use crate::VConnectIMServer;

#[derive(serde::Deserialize)]
pub struct HistoryQuery {
    pub uid: String,
    pub peer: Option<String>,
    pub since_ts: Option<i64>,
    pub until_ts: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(serde::Serialize, Debug)]
pub struct HistoryResponse {
    pub items: Vec<crate::storage::MessageRecord>,
}

// 路由注册入口（GET）/ Route registration (GET)
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(history_handle)));
}

// 历史消息查询 / Query message history
pub async fn history_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<HistoryQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(100);
    match server.storage.list_messages_by_user(
        &query.uid,
        query.peer.as_deref(),
        query.since_ts,
        query.until_ts,
        limit,
    ) {
        Ok(items) => respond_any(StatusCode::OK, HistoryResponse { items }),
        Err(e) => respond_any(StatusCode::BAD_REQUEST, serde_json::json!({ "error": format!("{}", e) })),
    }
}

