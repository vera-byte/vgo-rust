use crate::server::VConnectIMServer;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio::sync::watch;

pub fn spawn_cleanup_task(
    server: Arc<VConnectIMServer>,
    timeout_ms: u64,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let cleanup_interval_ms = if timeout_ms <= 1000 {
            timeout_ms / 2
        } else if timeout_ms <= 10000 {
            1000
        } else {
            5000
        };
        tracing::info!(
            "⏰ Cleanup interval set to {}ms for timeout {}ms",
            cleanup_interval_ms,
            timeout_ms
        );
        let mut cleanup_interval = interval(Duration::from_millis(cleanup_interval_ms));
        loop {
            tokio::select! {
                _ = cleanup_interval.tick() => {
                    server.cleanup_timeout_connections(timeout_ms).await;
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() { break; }
                }
            }
        }
    });
}
