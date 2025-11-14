use actix_web::web;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(login)));
}

use crate::service::user_service::user_login;
use v::http::HttpError;

pub async fn login() -> Result<web::Json<serde_json::Value>, HttpError> {
    let token = user_login()
        .await
        .map_err(|e| HttpError::Internal(format!("用户登录失败: {}", e)))?;
    Ok(web::Json(serde_json::json!({
        "message": "login success",
        "token": token,
    })))
}
