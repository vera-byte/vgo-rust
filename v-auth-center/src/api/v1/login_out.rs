use actix_web::web;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(login_out)));
}

use crate::service::user_service::user_login_out;
use v::http::HttpError;
use v::response;

pub async fn login_out() -> Result<web::Json<serde_json::Value>, HttpError> {
    let _ = user_login_out().await;
    let body = response::ok_body(&serde_json::json!({ "message": "login out success" }));
    Ok(web::Json(body))
}
