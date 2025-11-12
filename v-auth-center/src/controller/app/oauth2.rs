use actix_web::{web, HttpResponse, Responder};

pub const ROUTE_PATH: &str = "/app/auth";

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(handler)));
}

async fn handler() -> impl Responder {
    HttpResponse::Ok().body("auth")
}
