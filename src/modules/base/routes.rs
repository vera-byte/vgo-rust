use actix_web::{web, HttpResponse, Responder, Result};
use sa_token_plugin_actix_web::{sa_check_login, sa_check_permission, sa_check_role, sa_ignore};
use serde_json::json;

// 导入管理员登录相关的处理函数 - 暂时注释掉，这些函数还未实现
// use crate::modules::base::{admin_login, admin_logout, admin_logs, admin_profile, change_admin_password};
// 导入全局配置管理器
use crate::comm::config::get_global_config_manager;

/// 基础模块首页
#[actix_web::get("/base")]
#[sa_ignore]
pub async fn base_index() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({
        "message": "欢迎访问基础模块",
        "module": "base",
        "version": "1.0.0",
        "endpoints": [
            "/base",
            "/base/info",
            "/base/status",
            "/base/config",
            "/base/admin/settings"
        ]
    })))
}

/// 获取基础信息
#[actix_web::get("/base/info")]
#[sa_ignore]
pub async fn base_info() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({
        "module": "base",
        "description": "基础功能模块",
        "features": [
            "系统信息查询",
            "状态监控",
            "配置管理"
        ],
        "author": "VGO Team",
        "created_at": "2024-10-24"
    })))
}

/// 获取系统状态（需要登录）
#[actix_web::get("/base/status")]
#[sa_check_login]
pub async fn base_status() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "running",
        "uptime": "1h 30m",
        "memory_usage": "45%",
        "cpu_usage": "12%",
        "active_connections": 42,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// 获取配置信息（需要用户权限）
#[actix_web::get("/base/config")]
#[sa_check_permission("base:config:read")]
pub async fn base_config() -> Result<impl Responder> {
    // 使用全局配置管理器获取配置
    let config_manager = get_global_config_manager().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("获取配置管理器失败: {}", e))
    })?;

    // 获取配置值
    let debug_mode = config_manager.get_or("server.debug", false);
    let log_level = config_manager
        .get_string("logging.level")
        .unwrap_or("info".to_string());
    let max_connections = config_manager.get_or("server.max_connections", 1000i64);
    let timeout = config_manager.get_or("server.timeout", 30.0);

    Ok(HttpResponse::Ok().json(json!({
        "config": {
            "debug_mode": debug_mode,
            "log_level": log_level,
            "max_connections": max_connections,
            "timeout": timeout
        },
        "environment": "production",
        "last_updated": chrono::Utc::now().to_rfc3339()
    })))
}

/// 管理员设置（需要管理员角色）
#[actix_web::get("/base/admin/settings")]
#[sa_check_role("admin")]
pub async fn admin_settings() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({
        "settings": {
            "maintenance_mode": false,
            "backup_enabled": true,
            "monitoring_enabled": true,
            "security_level": "high"
        },
        "admin_info": {
            "last_login": "2024-10-24T09:15:00Z",
            "permissions": ["*"],
            "session_timeout": 3600
        }
    })))
}

/// 更新配置（POST请求，需要管理员权限）
#[actix_web::post("/base/config")]
#[sa_check_permission("base:config:write")]
pub async fn update_config(config: web::Json<serde_json::Value>) -> Result<impl Responder> {
    // 这里可以添加实际的配置更新逻辑
    Ok(HttpResponse::Ok().json(json!({
        "message": "配置更新成功",
        "updated_config": config.into_inner(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/*
pub fn configure_base_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(base_index)
        .service(base_info)
        .service(base_status)
        .service(base_config)
        .service(admin_settings)
        .service(update_config);
    // 管理员登录相关路由 - 暂时注释掉，这些函数还未实现
    // .service(admin_login)
    // .service(admin_logout)
    // .service(admin_profile)
    // .service(change_admin_password)
    // .service(admin_logs);
}
*/
