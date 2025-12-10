use actix_web::web;

/// 路由配置包装 / Route configuration wrapper
/// 只保留健康检查接口 / Only keep health check endpoints
pub fn configure(cfg: &mut web::ServiceConfig) {
    // 只注册健康检查接口 / Only register health check endpoints
    crate::api::v1::health::basic::register(cfg, "/v1/health");
    crate::api::v1::health::live::register(cfg, "/v1/health/live");
    crate::api::v1::health::ready::register(cfg, "/v1/health/ready");
    crate::api::v1::health::detailed::register(cfg, "/v1/health/detailed");
}
