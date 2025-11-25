use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use async_trait::async_trait;

use crate::comm::config::get_global_config_manager;
use crate::db::error::{DbError, Result};
use crate::{HealthCheck, HealthStatus};

lazy_static::lazy_static! {
    static ref POOLS: RwLock<HashMap<String, Pool<Postgres>>> = RwLock::new(HashMap::new());
}

/// 获取指定分组的 PostgreSQL 连接池（自动懒加载）
/// Get PostgreSQL pool for a group (lazy init)
///
/// 参数 / Params:
/// - `group`: 分库组名称 / database group name
///
/// 返回 / Returns: `Pool<Postgres>`
pub async fn get_pool(group: &str) -> Result<Pool<Postgres>> {
    if let Some(p) = POOLS.read().await.get(group).cloned() {
        return Ok(p);
    }
    let pool = build_pool(group).await?;
    POOLS.write().await.insert(group.to_string(), pool.clone());
    Ok(pool)
}

/// 根据配置构建连接池 / Build pool from configuration
///
/// 读取配置键 / Reads config keys:
/// - `database.<group>.url` 或 `host/port/user/pass/name/maxOpen`
async fn build_pool(group: &str) -> Result<Pool<Postgres>> {
    let mgr = get_global_config_manager().map_err(|e| DbError::Config(e.to_string()))?;
    let typ: String = mgr
        .get_or(
            &format!("database.{}.type", group),
            "postgresql".to_string(),
        )
        .to_lowercase();
    if typ != "postgresql" && typ != "postgres" {
        return Err(DbError::Config(format!(
            "不支持的数据库类型: {} (group={})",
            typ, group
        )));
    }

    let url_opt: Option<String> = mgr.get(&format!("database.{}.url", group)).ok();
    let max_open: u32 = mgr
        .get(&format!("database.{}.maxOpen", group))
        .map(|v: i64| v as u32)
        .unwrap_or(10);
    let host: String = mgr.get_or(&format!("database.{}.host", group), "127.0.0.1".to_string());
    let port: String = mgr.get_or(&format!("database.{}.port", group), "5432".to_string());
    let user: String = mgr.get_or(&format!("database.{}.user", group), "postgres".to_string());
    let pass: String = mgr.get_or(&format!("database.{}.pass", group), "".to_string());
    let name: String = mgr.get_or(&format!("database.{}.name", group), "postgres".to_string());
    let url = url_opt.unwrap_or_else(|| build_postgres_url(&host, &port, &user, &pass, &name));

    let pool = PgPoolOptions::new()
        .max_connections(max_open)
        .min_connections(1)
        .max_lifetime(Some(Duration::from_secs(1800)))
        .idle_timeout(Some(Duration::from_secs(300)))
        .acquire_timeout(Duration::from_secs(3))
        .connect(&url)
        .await
        .map_err(DbError::from)?;
    Ok(pool)
}

/// 构建 PostgreSQL 连接 URL / Build PostgreSQL URL
///
/// 示例 / Example: `postgres://user:pass@host:port/db`
pub fn build_postgres_url(host: &str, port: &str, user: &str, pass: &str, name: &str) -> String {
    let enc_user = urlencoding::encode(user);
    let enc_pass = urlencoding::encode(pass);
    format!(
        "postgres://{}:{}@{}:{}/{}",
        enc_user, enc_pass, host, port, name
    )
}

/// 健康检查 / Health check
///
/// 执行 `SELECT 1` 验证连接可用 / runs `SELECT 1`
pub async fn check_health(pool: &Pool<Postgres>) -> Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(DbError::from)
}

/// 为 PostgreSQL 连接池实现通用健康检查接口
/// Implement generic HealthCheck interface for PostgreSQL pool
#[async_trait]
impl HealthCheck for Pool<Postgres> {
    async fn check_health(&self) -> HealthStatus {
        // 执行轻量查询以验证连接
        // Run a lightweight query to validate connectivity
        let res = sqlx::query("SELECT 1").execute(self).await;
        match res {
            Ok(_) => HealthStatus {
                component: "postgres_pool".to_string(),
                healthy: true,
                message: Some("OK".to_string()),
                timestamp: chrono::Utc::now(),
            },
            Err(e) => HealthStatus {
                component: "postgres_pool".to_string(),
                healthy: false,
                message: Some(format!("SQLx error: {}", e)),
                timestamp: chrono::Utc::now(),
            },
        }
    }
}

/// 开启事务 / Begin transaction
///
/// 返回 `sqlx::Transaction` 以进行提交/回滚 / returns transaction handle
pub async fn begin_tx(pool: &Pool<Postgres>) -> Result<sqlx::Transaction<'_, Postgres>> {
    pool.begin().await.map_err(DbError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let u = build_postgres_url("localhost", "5432", "u@x", "p:wd", "db");
        assert!(u.starts_with("postgres://"));
        assert!(u.contains("localhost:5432/db"));
        assert!(u.contains("u%40x"));
        assert!(u.contains("p%3Awd"));
    }

    #[tokio::test]
    async fn test_pool_and_health() {
        std::env::set_var("V_DATABASE_DEFAULT_TYPE", "postgresql");
        std::env::set_var("V_DATABASE_DEFAULT_HOST", "127.0.0.1");
        std::env::set_var("V_DATABASE_DEFAULT_PORT", "5432");
        std::env::set_var("V_DATABASE_DEFAULT_USER", "postgres");
        std::env::set_var("V_DATABASE_DEFAULT_PASS", "");
        std::env::set_var("V_DATABASE_DEFAULT_NAME", "postgres");
        let p = get_pool("default").await.unwrap();
        let _ = check_health(&p).await; // may fail if db not running, should not panic

        // another group routing
        std::env::set_var("V_DATABASE_AUDIT_TYPE", "postgresql");
        std::env::set_var("V_DATABASE_AUDIT_HOST", "127.0.0.1");
        std::env::set_var("V_DATABASE_AUDIT_PORT", "5432");
        std::env::set_var("V_DATABASE_AUDIT_USER", "postgres");
        std::env::set_var("V_DATABASE_AUDIT_PASS", "");
        std::env::set_var("V_DATABASE_AUDIT_NAME", "postgres");
        let p2 = get_pool("audit").await.unwrap();
        let _ = check_health(&p2).await;
    }
}
