// v 库主入口，按需导出模块

pub mod comm;
pub use crate::comm::config::*;
pub use crate::comm::geo::*;
pub use crate::comm::tracing::init_tracing;

pub mod db;
pub use crate::db::connection::*;
pub use crate::db::error::*;
pub use crate::db::model::*;
pub use crate::db::query::*;
pub mod response;

// 导出通用仓库 Trait
pub mod repo;
pub use crate::repo::*;

pub mod plugin;

// 重新导出 tracing 宏，方便插件开发
// Re-export tracing macros for plugin development convenience
pub use tracing::{debug, error, info, trace, warn};

// 健康检查接口与状态定义（统一对外暴露）
// Health check interface and status definitions (unified public exposure)
use async_trait::async_trait;

/// 健康状态结构体：用于表示组件当前健康状况
/// Health status struct: represents the current health of a component
#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    /// 组件名称（如 postgres_pool、redis_cache）
    /// Component name (e.g., postgres_pool, redis_cache)
    pub component: String,
    /// 是否健康（true=健康，false=不健康）
    /// Whether healthy (true=healthy, false=unhealthy)
    pub healthy: bool,
    /// 附加消息（错误信息或提示）
    /// Additional message (error details or hint)
    pub message: Option<String>,
    /// 采样时间戳（UTC）
    /// Sample timestamp (UTC)
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 健康检查通用接口：由各服务或资源实现具体检查逻辑
/// Generic health check interface: implemented by services/resources with concrete logic
#[async_trait]
pub trait HealthCheck {
    /// 执行健康检查并返回健康状态
    /// Perform health check and return the status
    async fn check_health(&self) -> HealthStatus;
}
