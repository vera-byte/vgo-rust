use actix_web::{guard, middleware, web, HttpResponse};
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(
        web::resource("/user/{name}") // 动态路径
            .name("user_detail") // 资源名称，用于 URL 生成
            .guard(guard::Header("content-type", "application/json")) // 守卫条件
            .route(web::get().to(HttpResponse::Ok)) // GET 方法
            .route(web::put().to(HttpResponse::Ok)), // PUT 方法
    );
}
