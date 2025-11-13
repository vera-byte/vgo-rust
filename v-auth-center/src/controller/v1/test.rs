use actix_web::web;
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(test)));
}

use v::http::HttpError;

pub async fn test() -> Result<web::Json<serde_json::Value>, HttpError> {
    Ok(web::Json(serde_json::json!({
        "message": "test",
    })))
}
