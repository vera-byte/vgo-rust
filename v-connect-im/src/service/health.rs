use crate::server::VConnectIMServer;
use async_trait::async_trait;
use v::{HealthCheck, HealthStatus};

// 为 IM 服务实现统一健康检查接口
// Implement unified HealthCheck for IM service
#[async_trait]
impl HealthCheck for VConnectIMServer {
    /// 执行IM服务的健康检查（连接数等）
    /// Perform health check for IM service (connection count, etc.)
    async fn check_health(&self) -> HealthStatus {
        let online_count = self.connections.len();
        let healthy = online_count < 10_000;
        // Webhook 已移除 / Webhook removed
        let msg = Some(format!("online={}", online_count));

        HealthStatus {
            component: "im_server".to_string(),
            healthy,
            message: msg,
            timestamp: chrono::Utc::now(),
        }
    }
}
