use std::any::Any;
use thiserror::Error;
pub struct BaseService;

#[derive(Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

impl BaseService {
    pub fn new() -> Self {
        Self {}
    }
    /// 密码登录
    /// Login by password
    pub async fn login_by_password(req: &LoginRequest) -> Result<Box<dyn Any>, ServiceError> {
        let _ = req;
        Ok(Box::new(()))
    }
}
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("登录失败: {0}")]
    LoginError(String),
}
