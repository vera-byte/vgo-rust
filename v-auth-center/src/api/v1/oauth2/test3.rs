use actix_web::{web, Result};
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::get().to(test3)));
}

#[derive(serde::Deserialize)]
struct TestReq {
    name: String,
}
async fn test3(info: web::Json<TestReq>) -> Result<String> {
    Ok(format!("Welcome {}!", info.name))
}
