use actix_web::{web, HttpResponse, Responder};

pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(handler)));
}

async fn handler() -> impl Responder {
    HttpResponse::Ok().body("login")
}
