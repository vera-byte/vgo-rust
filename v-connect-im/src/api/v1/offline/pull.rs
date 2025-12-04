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

    // 通过存储插件拉取离线消息 / Pull offline messages through storage plugin
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool.storage_pull_offline(&query.uid, limit).await {
            Ok(messages) => {
                // 将 JSON 值转换为 OfflineRecord / Convert JSON values to OfflineRecord
                let items: Vec<crate::storage::OfflineRecord> = messages
                    .iter()
                    .filter_map(|msg| serde_json::from_value(msg.clone()).ok())
                    .collect();

                respond_any(
                    StatusCode::OK,
                    PullResponse {
                        items,
                        next_cursor: None, // 简化版本，不支持游标 / Simplified, no cursor support
                    },
                )
            }
            Err(e) => respond_any(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": format!("存储插件错误 / Storage plugin error: {}", e) }),
            ),
        }
    } else {
        respond_any(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "存储插件未初始化 / Storage plugin not initialized" }),
        )
    }
}
