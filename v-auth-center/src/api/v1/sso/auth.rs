use actix_web::{http, web, Responder};

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(sso_auth_handle)));
}
pub async fn sso_auth_handle() -> impl Responder {
    return v::response::respond_any(http::StatusCode::OK, "sso_auth_handle");
}
