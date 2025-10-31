use actix_web::{web, HttpResponse, Responder, Result};
use sa_token_plugin_actix_web::{sa_check_login, sa_check_role, sa_ignore, LoginIdExtractor};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 管理员登录请求结构体
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
    pub remember_me: Option<bool>,
}

/// 管理员登录响应结构体
#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
    pub user_info: AdminUserInfo,
    pub expires_in: i64,
}

/// 管理员用户信息结构体
#[derive(Debug, Serialize)]
pub struct AdminUserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub last_login: Option<String>,
}

/// 管理员登录
#[actix_web::post("/base/admin/login")]
#[sa_ignore]
pub async fn admin_login(req: web::Json<AdminLoginRequest>) -> Result<impl Responder> {
    // 这里应该实现真实的登录逻辑，包括密码验证、token生成等
    // 目前提供一个模拟实现

    let username = &req.username;
    let password = &req.password;

    // 模拟验证管理员账户
    if username == "admin" && password == "admin123" {
        let response = AdminLoginResponse {
            token: format!("admin_token_{}", chrono::Utc::now().timestamp()),
            user_info: AdminUserInfo {
                id: "admin_001".to_string(),
                username: username.clone(),
                role: "admin".to_string(),
                permissions: vec![
                    "admin:read".to_string(),
                    "admin:write".to_string(),
                    "user:manage".to_string(),
                    "system:config".to_string(),
                ],
                last_login: Some(chrono::Utc::now().to_rfc3339()),
            },
            expires_in: 3600, // 1小时
        };

        Ok(HttpResponse::Ok().json(json!({
            "code": 200,
            "message": "管理员登录成功",
            "data": response
        })))
    } else {
        Ok(HttpResponse::Unauthorized().json(json!({
            "code": 401,
            "message": "用户名或密码错误",
            "data": null
        })))
    }
}

/// 管理员登出
#[actix_web::post("/base/admin/logout")]
#[sa_check_login]
pub async fn admin_logout(login_id: LoginIdExtractor) -> Result<impl Responder> {
    // 这里应该实现真实的登出逻辑，如清除token等

    Ok(HttpResponse::Ok().json(json!({
        "code": 200,
        "message": "管理员登出成功",
        "data": {
            "user_id": login_id.0,
            "logout_time": chrono::Utc::now().to_rfc3339()
        }
    })))
}

/// 获取当前管理员信息
#[actix_web::get("/base/admin/profile")]
#[sa_check_role("admin")]
pub async fn admin_profile(login_id: LoginIdExtractor) -> Result<impl Responder> {
    // 模拟获取管理员信息
    let admin_info = AdminUserInfo {
        id: login_id.0.clone(),
        username: "admin".to_string(),
        role: "admin".to_string(),
        permissions: vec![
            "admin:read".to_string(),
            "admin:write".to_string(),
            "user:manage".to_string(),
            "system:config".to_string(),
        ],
        last_login: Some(chrono::Utc::now().to_rfc3339()),
    };

    Ok(HttpResponse::Ok().json(json!({
        "code": 200,
        "message": "获取管理员信息成功",
        "data": admin_info
    })))
}

/// 修改管理员密码
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[actix_web::put("/base/admin/password")]
#[sa_check_role("admin")]
pub async fn change_admin_password(
    req: web::Json<ChangePasswordRequest>,
    login_id: LoginIdExtractor,
) -> Result<impl Responder> {
    // 这里应该实现真实的密码修改逻辑

    // 模拟验证旧密码
    if req.old_password == "admin123" {
        Ok(HttpResponse::Ok().json(json!({
            "code": 200,
            "message": "密码修改成功",
            "data": {
                "user_id": login_id.0,
                "updated_at": chrono::Utc::now().to_rfc3339()
            }
        })))
    } else {
        Ok(HttpResponse::BadRequest().json(json!({
            "code": 400,
            "message": "旧密码错误",
            "data": null
        })))
    }
}

/// 获取管理员操作日志
#[actix_web::get("/base/admin/logs")]
#[sa_check_role("admin")]
pub async fn admin_logs() -> Result<impl Responder> {
    // 模拟管理员操作日志
    let logs = vec![
        json!({
            "id": 1,
            "action": "login",
            "description": "管理员登录系统",
            "ip": "192.168.1.100",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
        json!({
            "id": 2,
            "action": "config_update",
            "description": "更新系统配置",
            "ip": "192.168.1.100",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
    ];

    Ok(HttpResponse::Ok().json(json!({
        "code": 200,
        "message": "获取操作日志成功",
        "data": {
            "logs": logs,
            "total": 2,
            "page": 1,
            "page_size": 10
        }
    })))
}
