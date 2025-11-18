use crate::model::user::User;
use crate::repo::user_repo::{self};
use anyhow::Result;
use sa_token_plugin_actix_web::StpUtil;
use serde_json::json;

pub async fn get_user(id: i64) -> Result<Option<User>> {
    user_repo::info(id).await
}

pub async fn list_users() -> Result<Vec<User>> {
    user_repo::list().await
}

pub async fn user_login() -> Result<String> {
    let permissions = vec!["user:read".to_string(), "user:write".to_string()];
    let _ = StpUtil::set_permissions("new_user_456", permissions.clone()).await;
    let per_result = StpUtil::get_permissions("new_user_456").await;
    tracing::info!("get_permissions result: {:?}", per_result);
    let token = StpUtil::builder("new_user_456")
        .extra_data(json!({"ip": "192.168.1.1"}))
        .device("pc")
        .login_type("admin")
        .login(Some("new_user_456")) // 或 Some(10001) 数字ID
        .await?;
    Ok(token.to_string())
}

pub async fn user_login_out() -> Result<()> {
    StpUtil::kick_out("new_user_456").await?;
    Ok(())
}
