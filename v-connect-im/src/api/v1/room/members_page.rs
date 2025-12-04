use crate::VConnectIMServer;
use actix_web::http::StatusCode;
use actix_web::{web, Responder};
use std::sync::Arc;
use v::response::respond_any;

#[derive(serde::Deserialize)]
pub struct MembersPageQuery {
    pub room_id: String,
    pub uid_prefix: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(serde::Serialize, Debug)]
pub struct MembersPageResponse {
    pub room_id: String,
    pub members: Vec<String>,
    pub next_cursor: Option<String>,
}

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(members_page_handle)));
}

pub async fn members_page_handle(
    _server: web::Data<Arc<VConnectIMServer>>,
    _query: web::Query<MembersPageQuery>,
) -> impl Responder {
    // 房间成员分页功能已迁移到插件 / Room members pagination feature migrated to plugin
    respond_any(
        StatusCode::SERVICE_UNAVAILABLE,
        serde_json::json!({
            "error": "Feature migrated to plugin",
            "message": "房间成员分页需要存储插件 / Room members pagination requires storage plugin"
        }),
    )
}
