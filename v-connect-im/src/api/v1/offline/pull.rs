use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct PullQuery {
    pub uid: String,            // 用户ID / User ID
    pub limit: Option<usize>,   // 条数限制 / Limit
    pub cursor: Option<String>, // 游标（uid:timestamp:msgid）/ Cursor
    pub since_ts: Option<i64>,  // 起始时间戳 / Since timestamp
    pub until_ts: Option<i64>,  // 结束时间戳 / Until timestamp
}

#[derive(serde::Serialize, Debug)]
pub struct PullResponse {
    pub items: Vec<crate::storage::OfflineRecord>, // 离线消息列表 / Offline items
    pub next_cursor: Option<String>,               // 下一页游标 / Next cursor
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(offline_pull_handle)));
}

pub async fn offline_pull_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<PullQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(100);
    match server.storage.pull_offline_by_time(
        &query.uid,
        query.cursor.clone(),
        limit,
        query.since_ts,
        query.until_ts,
    ) {
        Ok((items, next_cursor)) => {
            respond_any(StatusCode::OK, PullResponse { items, next_cursor })
        }
        Err(e) => respond_any(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "error": format!("{}", e) }),
        ),
    }
}
