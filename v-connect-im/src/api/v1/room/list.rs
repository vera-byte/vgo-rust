use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Serialize, Debug)]
pub struct RoomListResponse {
    pub rooms: Vec<String>,
    pub count: usize,
}

#[derive(serde::Deserialize)]
pub struct RoomListQuery {
    pub prefix: Option<String>,
    pub limit: Option<usize>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(room_list_handle)));
}

pub async fn room_list_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    _query: web::Query<RoomListQuery>,
) -> impl Responder {
    // 通过存储插件列出所有房间 / List all rooms through storage plugin
    // 注意：简化版本不支持 prefix 和 limit 参数 / Note: Simplified version doesn't support prefix and limit
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool.storage_list_rooms().await {
            Ok(rooms) => respond_any(
                StatusCode::OK,
                RoomListResponse {
                    count: rooms.len(),
                    rooms,
                },
            ),
            Err(e) => respond_any(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "error": format!("存储插件错误 / Storage plugin error: {}", e)
                }),
            ),
        }
    } else {
        respond_any(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({
                "error": "存储插件未初始化 / Storage plugin not initialized"
            }),
        )
    }
}
