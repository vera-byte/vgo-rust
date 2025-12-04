use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct MembersQuery {
    pub room_id: String,
}

#[derive(serde::Serialize, Debug)]
pub struct MembersResponse {
    pub room_id: String,
    pub members: Vec<String>,
    pub count: usize,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(room_members_handle)));
}

pub async fn room_members_handle(
    server: web::Data<Arc<VConnectIMServer>>,
    query: web::Query<MembersQuery>,
) -> impl Responder {
    // 通过存储插件列出房间成员 / List room members through storage plugin
    if let Some(pool) = server.plugin_connection_pool.as_ref() {
        match pool.storage_list_room_members(&query.room_id).await {
            Ok(members) => respond_any(
                StatusCode::OK,
                MembersResponse {
                    room_id: query.room_id.clone(),
                    count: members.len(),
                    members,
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
