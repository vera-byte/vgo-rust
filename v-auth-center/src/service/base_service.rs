use sa_token_plugin_actix_web::StpUtil;
use serde::Serialize;
use serde_json::json;

use crate::errors::BaseServerError;
pub type BaseLoginResult = Result<LoginResponse, BaseServerError>;

pub struct BaseService;

#[derive(Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
}

impl BaseService {
    pub fn new() -> Self {
        Self {}
    }
    /// 密码登录
    /// Login by password
    pub async fn login_by_password(req: &LoginRequest) -> BaseLoginResult {
        // 1.校验用户名密码是否为空
        if req.username.is_empty() || req.password.is_empty() {
            return Err(BaseServerError::LoginError(
                "用户名或密码不能为空".to_string(),
            ));
        }
        // 2. 校验用户名密码是否正确

        // 2.调用登录
        let token = StpUtil::builder(&req.username)
            .extra_data(json!({
                "username": req.username.clone(),
                "password": req.password.clone(),
            }))
            .device("pc")
            .login_type("admin")
            .login(Some(req.username.clone()))
            .await
            .map_err(|e| BaseServerError::LoginError(format!("登录失败: {}", e)))?;

        let is_login = StpUtil::check_login(&token).await;
        if is_login.is_ok() {
            StpUtil::set_session_value(&req.username, "is_vip", true).await;
            tracing::info!("登录成功");
        }
        // 返回登录成功响应
        Ok(LoginResponse {
            access_token: token.to_string(),
        })
    }
}
