use actix_web::web;

/// 路由配置包装 / Route configuration wrapper
pub fn configure(cfg: &mut web::ServiceConfig) {
    crate::api_registry::configure(cfg);
}

