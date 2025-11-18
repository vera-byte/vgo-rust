use actix_web::{get, Responder};
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(test_route);
}

#[get("/hello")]
pub async fn test_route() -> impl Responder {
    "hello world"
}
