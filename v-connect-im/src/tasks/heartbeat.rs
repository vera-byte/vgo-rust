use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::server::VConnectIMServer;

/// 启动心跳清理任务 / Spawn heartbeat cleanup task
pub fn spawn_cleanup_task(server: Arc<VConnectIMServer>, timeout_ms: u64) {
    tokio::spawn(async move {
        let cleanup_interval_ms = if timeout_ms <= 1000 { timeout_ms / 2 } else if timeout_ms <= 10000 { 1000 } else { 5000 };
        tracing::info!("⏰ Cleanup interval set to {}ms for timeout {}ms", cleanup_interval_ms, timeout_ms);
        let mut cleanup_interval = interval(Duration::from_millis(cleanup_interval_ms));
        loop { cleanup_interval.tick().await; server.cleanup_timeout_connections(timeout_ms).await; }
    });
}

