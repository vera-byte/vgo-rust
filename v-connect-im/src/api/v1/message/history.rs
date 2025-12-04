use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

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

    // 优先使用存储插件查询 / Prefer storage plugin query
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool
            .storage_query_history(
                Some(&query.uid),
                query.peer.as_deref(),
                query.since_ts,
                query.until_ts,
                limit,
            )
            .await
        {
            Ok(messages) => {
                return respond_any(
                    StatusCode::OK,
                    serde_json::json!({
                        "messages": messages,
                        "count": messages.len()
                    }),
                );
            }
            Err(e) => {
                tracing::warn!("存储插件查询失败，回退到本地存储 / Storage plugin query failed, fallback to local: {}", e);
            }
        }
    }

    // 本地存储已移除，只使用插件 / Local storage removed, plugin only
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Storage plugin not available",
            "message": "历史消息功能需要存储插件 / History feature requires storage plugin"
        }),
    )
}
