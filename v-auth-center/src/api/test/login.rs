use actix_web::web;
use serde::{Deserialize, Serialize};
pub fn register(cfg: &mut actix_web::web::ServiceConfig, path: &str) {
    cfg.service(web::resource(path).route(web::post().to(login)));
}
use sa_token_plugin_actix_web::{SaTokenState, StpUtil};
use v::http::HttpError;
// ==================== 请求/响应类型 ====================
// ==================== Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_info: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ManageUserRequest {
    pub user_id: String,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct AddPermissionRequest {
    pub user_id: String,
    pub permission: String,
}

#[derive(Debug, Deserialize)]
pub struct RemovePermissionRequest {
    pub user_id: String,
    pub permission: String,
}

// ==================== 登录接口 ====================
// ==================== Login Endpoint ====================

pub async fn login(
    state: web::Data<SaTokenState>,
    req: web::Json<LoginRequest>,
) -> Result<web::Json<LoginResponse>, HttpError> {
    // 验证用户名密码（实际应该查询数据库）
    // Validate username and password (should query database in real application)
    let user_id = match req.username.as_str() {
        "admin" if req.password == "admin123" => "admin",
        "user" if req.password == "user123" => "user",
        "guest" if req.password == "guest123" => "guest",
        _ => {
            return Err(HttpError::Unauthorized("用户名或密码错误".to_string()));
        }
    };

    // 生成token - 使用注入的 sa_token 状态
    // Generate token - using injected sa_token state
    let token = state
        .manager
        .login(user_id)
        .await
        .map_err(|e| HttpError::Internal(format!("登录失败 / Login failed: {}", e)))?;

    // 获取用户权限和角色（使用 StpUtil）
    // Get user permissions and roles (using StpUtil)
    let permissions = StpUtil::get_permissions(user_id).await;
    let roles = StpUtil::get_roles(user_id).await;

    tracing::info!(
        "✅ 用户 {} 登录成功，权限: {:?}, 角色: {:?}",
        user_id,
        permissions,
        roles
    );
    tracing::info!(
        "✅ User {} logged in successfully, permissions: {:?}, roles: {:?}",
        user_id,
        permissions,
        roles
    );

    let user_info = UserInfo {
        id: user_id.to_string(),
        username: req.username.clone(),
        nickname: match user_id {
            "admin" => "管理员",
            "user" => "普通用户",
            "guest" => "访客",
            _ => "未知",
        }
        .to_string(),
        email: Some(format!("{}@example.com", req.username)),
    };

    let response = LoginResponse {
        token: token.to_string(),
        user_info,
    };

    Ok(web::Json(response))
}
