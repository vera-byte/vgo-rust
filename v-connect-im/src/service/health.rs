use async_trait::async_trait;
use crate::VConnectIMServer;
use v::{HealthCheck, HealthStatus};

// 为 IM 服务实现统一健康检查接口
// Implement unified HealthCheck for IM service
#[async_trait]
impl HealthCheck for VConnectIMServer {
    /// 执行IM服务的健康检查（连接数、Webhook启用状态等）
    /// Perform health check for IM service (connection count, webhook status)
    async fn check_health(&self) -> HealthStatus {
        let online_count = self.connections.len();
        let healthy = online_count < 10_000;
        let msg = if let Some(cfg) = &self.webhook_config {
            Some(format!(
                "online={} webhook_enabled={} url={}",
                online_count, cfg.enabled, cfg.url
            ))
        } else {
            Some(format!("online={}", online_count))
        };

        HealthStatus {
            component: "im_server".to_string(),
            healthy,
            message: msg,
            timestamp: chrono::Utc::now(),
        }
    }
}
