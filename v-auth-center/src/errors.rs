use thiserror::Error;

#[derive(Debug, Error)]
pub enum BaseServerError {
    #[error("登录失败: {0}")]
    LoginError(String),
}
