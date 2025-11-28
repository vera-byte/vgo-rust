use crate::plugins::Plugin;
use crate::server::VConnectIMServer;
use anyhow::Result;
use async_trait::async_trait;
use v::plugin::builtin::auth::{AuthPlugin as VAuthPlugin, DefaultAuthPlugin as VDefaultAuthPlugin};

/// 授权插件接口 / Authorization plugin interface
#[async_trait]
pub trait AuthPlugin: Plugin {
    async fn validate(&self, server: &VConnectIMServer, token: &str) -> Result<bool>;
    async fn apply(&self, server: &VConnectIMServer, client_id: &str, uid: &str) -> Result<()>;
}

/// 默认内置授权插件 / Built-in authorization plugin
pub struct DefaultAuthPlugin { inner: VDefaultAuthPlugin }

impl DefaultAuthPlugin { pub fn new() -> Self { Self { inner: VDefaultAuthPlugin::new() } } }

impl Plugin for DefaultAuthPlugin {
    fn name(&self) -> &'static str {
        "builtin-auth"
    }
}

#[async_trait]
impl AuthPlugin for DefaultAuthPlugin {
    async fn validate(&self, server: &VConnectIMServer, token: &str) -> Result<bool> {
        VAuthPlugin::validate(&self.inner, server, token).await
    }
    async fn apply(&self, server: &VConnectIMServer, client_id: &str, uid: &str) -> Result<()> {
        VAuthPlugin::apply(&self.inner, server, client_id, uid).await
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
