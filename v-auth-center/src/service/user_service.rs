use crate::model::user::User;
use crate::repo::user_repo::{self};
use sa_token_plugin_actix_web::StpUtil;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("数据库错误: {0}")]
    Db(#[from] v::db::error::DbError),
    #[error("Sa-Token 错误: {0}")]
    SaToken(String),
}

pub async fn get_user(id: i64) -> Result<Option<User>, ServiceError> {
    user_repo::info(id).await.map_err(ServiceError::from)
}

pub async fn list_users() -> Result<Vec<User>, ServiceError> {
    user_repo::list().await.map_err(ServiceError::from)
}

pub async fn user_login() -> Result<String, ServiceError> {
    let permissions = vec!["user:read".to_string(), "user:write".to_string()];
    let _ = StpUtil::set_permissions("new_user_456", permissions.clone())
        .await
        .map_err(|e| ServiceError::SaToken(e.to_string()))?;
    let per_result = StpUtil::get_permissions("new_user_456").await;
    tracing::info!("get_permissions result: {:?}", per_result);
    let token = StpUtil::builder("new_user_456")
        .extra_data(json!({"ip": "192.168.1.1"}))
        .device("pc")
        .login_type("admin")
        .login(Some("new_user_456")) // 或 Some(10001) 数字ID
        .await
        .map_err(|e| ServiceError::SaToken(e.to_string()))?;
    Ok(token.to_string())
}

pub async fn user_login_out() -> Result<(), ServiceError> {
    StpUtil::kick_out("new_user_456")
        .await
        .map_err(|e| ServiceError::SaToken(e.to_string()))?;
    Ok(())
}
