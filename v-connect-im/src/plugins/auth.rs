use crate::plugins::Plugin;
use crate::server::VConnectIMServer;
use anyhow::Result;
use async_trait::async_trait;

/// 授权插件接口 / Authorization plugin interface
#[async_trait]
pub trait AuthPlugin: Plugin {
    /// 验证客户端令牌 / Validate client token
    async fn validate(&self, server: &VConnectIMServer, token: &str) -> Result<bool>;

    /// 应用授权结果 / Apply authorization result
    async fn apply(&self, server: &VConnectIMServer, client_id: &str, uid: &str) -> Result<()>;
}

/// 默认内置授权插件 / Built-in authorization plugin
pub struct DefaultAuthPlugin;

impl DefaultAuthPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for DefaultAuthPlugin {
    fn name(&self) -> &'static str {
        "builtin-auth"
    }
}

#[async_trait]
impl AuthPlugin for DefaultAuthPlugin {
    async fn validate(&self, server: &VConnectIMServer, token: &str) -> Result<bool> {
        if token.is_empty() {
            return Ok(false);
        }
        if let Some(cfg) = &server.auth_config {
            if !cfg.enabled {
                return Ok(true);
            }
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(cfg.timeout_ms))
                .build()?;
            let resp = client
                .get(format!("{}/v1/sso/auth", cfg.center_url))
                .query(&[("token", token)])
                .send()
                .await?;
            Ok(resp.status().is_success())
        } else {
            Ok(true)
        }
    }

    async fn apply(&self, server: &VConnectIMServer, client_id: &str, uid: &str) -> Result<()> {
        if let Some(mut conn) = server.connections.get_mut(client_id) {
            conn.uid = Some(uid.to_string());
        }
        // 检查是否是该用户的首个连接 / Check if this is the first connection for this user
        let is_first_connection = server
            .uid_clients
            .get(uid)
            .map(|set| set.is_empty())
            .unwrap_or(true);

        let set = server.uid_clients.entry(uid.to_string()).or_default();
        set.insert(client_id.to_string());

        // 如果是首个连接，触发 user.online 事件 / Emit user.online if first connection
        if is_first_connection {
            let event = serde_json::json!({
                "uid": uid,
                "client_id": client_id,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            });
            if let Err(e) = server
                .plugin_registry
                .emit_custom("user.online", &event)
                .await
            {
                tracing::warn!("plugin user.online event error: {}", e);
            }
        }

        let _ = server.deliver_offline_for_uid(uid, client_id).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{Connection, VConnectIMServer};
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::Message;

    #[tokio::test]
    async fn validate_without_config_returns_true() {
        let server = VConnectIMServer::new();
        let plugin = DefaultAuthPlugin::new();
        let ok = plugin.validate(&server, "token").await.unwrap();
        assert!(ok);
    }

    #[tokio::test]
    async fn apply_sets_uid_and_mappings() {
        let server = VConnectIMServer::new();
        let (tx, _rx) = mpsc::unbounded_channel::<Message>();
        let conn = Connection {
            client_id: "c1".to_string(),
            uid: None,
            addr: "127.0.0.1:9000".parse::<SocketAddr>().unwrap(),
            sender: tx,
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
        };
        server.connections.insert("c1".to_string(), conn);

        let plugin = DefaultAuthPlugin::new();
        plugin.apply(&server, "c1", "u1").await.unwrap();

        let stored_uid = server
            .connections
            .get("c1")
            .and_then(|c| c.uid.clone())
            .unwrap();
        assert_eq!(stored_uid, "u1");
        assert!(server
            .uid_clients
            .get("u1")
            .map(|set| set.contains("c1"))
            .unwrap_or(false));
    }
}
