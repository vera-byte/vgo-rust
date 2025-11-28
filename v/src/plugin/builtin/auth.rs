use anyhow::Result;
use async_trait::async_trait;

use crate::plugin::types::ImServer;

/// 授权插件接口 / Authorization plugin interface
#[async_trait]
pub trait AuthPlugin<S: ImServer>: Send + Sync {
    async fn validate(&self, server: &S, token: &str) -> Result<bool>;
    async fn apply(&self, server: &S, client_id: &str, uid: &str) -> Result<()>;
}

/// 默认授权插件 / Default auth plugin
pub struct DefaultAuthPlugin;

impl DefaultAuthPlugin { pub fn new() -> Self { Self } }

#[async_trait]
impl<S: ImServer> AuthPlugin<S> for DefaultAuthPlugin {
    async fn validate(&self, server: &S, token: &str) -> Result<bool> {
        if token.is_empty() { return Ok(false); }
        if let Some(cfg) = server.auth_config() {
            if !cfg.enabled() { return Ok(true); }
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(cfg.timeout_ms()))
                .build()?;
            let resp = client
                .get(format!("{}/v1/sso/auth", cfg.center_url()))
                .query(&[("token", token)])
                .send()
                .await?;
            Ok(resp.status().is_success())
        } else {
            Ok(true)
        }
    }

    async fn apply(&self, server: &S, client_id: &str, uid: &str) -> Result<()> {
        server.set_connection_uid(client_id, uid);

        let is_first_connection = server.uid_clients_is_empty(uid);
        server.uid_clients_insert(uid, client_id);

        if is_first_connection {
            let event = serde_json::json!({
                "uid": uid,
                "client_id": client_id,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            });
            let _ = server.emit_custom("user.online", &event).await;
        }

        let _ = server.deliver_offline_for_uid(uid, client_id).await;
        Ok(())
    }
}
