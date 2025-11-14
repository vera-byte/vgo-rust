use actix_web::web;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource("/v1/test/user").route(web::get().to(test)));
}

use crate::service::user_service;
use v::http::HttpError;

pub async fn test() -> Result<web::Json<serde_json::Value>, HttpError> {
    let users = user_service::list_users()
        .await
        .map_err(|e| HttpError::Internal(format!("查询用户失败: {}", e)))?;
    Ok(web::Json(serde_json::json!({
        "message": "test",
        "users": users,
    })))
}
