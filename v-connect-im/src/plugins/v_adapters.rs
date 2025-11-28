use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::domain::message::ImMessage;
use crate::plugins::PluginContext;
use crate::server::VConnectIMServer;

use v::plugin::types::{AuthConfigLiteAccess, ImMessageAccess, ImServer, PluginContextAccess};

impl ImMessageAccess for ImMessage {
    fn msg_type(&self) -> &str { &self.msg_type }
    fn set_msg_type(&mut self, s: String) { self.msg_type = s; }
    fn data(&self) -> &Value { &self.data }
    fn set_data(&mut self, v: Value) { self.data = v; }
    fn target_uid(&self) -> Option<&str> { self.target_uid.as_deref() }
    fn set_target_uid(&mut self, uid: Option<String>) { self.target_uid = uid; }
}

impl<'a> PluginContextAccess<VConnectIMServer> for PluginContext<'a> {
    fn client_id(&self) -> &str { self.client_id }
    fn server(&self) -> Option<&VConnectIMServer> { self.server }
}

impl AuthConfigLiteAccess for crate::config::AuthConfigLite {
    fn enabled(&self) -> bool { self.enabled }
    fn center_url(&self) -> &str { &self.center_url }
    fn timeout_ms(&self) -> u64 { self.timeout_ms }
}

#[async_trait]
impl ImServer for VConnectIMServer {
    fn node_id(&self) -> &str { &self.node_id }
    fn auth_config(&self) -> Option<&dyn AuthConfigLiteAccess> {
        self.auth_config.as_ref().map(|c| c as &dyn AuthConfigLiteAccess)
    }
    fn set_connection_uid(&self, client_id: &str, uid: &str) {
        if let Some(mut conn) = self.connections.get_mut(client_id) {
            conn.uid = Some(uid.to_string());
        }
    }
    fn uid_clients_is_empty(&self, uid: &str) -> bool {
        self.uid_clients.get(uid).map(|set| set.is_empty()).unwrap_or(true)
    }
    fn uid_clients_insert(&self, uid: &str, client_id: &str) {
        let set = self.uid_clients.entry(uid.to_string()).or_default();
        set.insert(client_id.to_string());
    }
    async fn emit_custom(&self, event_type: &str, payload: &Value) -> Result<()> {
        self.plugin_registry.emit_custom(event_type, payload).await
    }
    async fn deliver_offline_for_uid(&self, uid: &str, client_id: &str) -> Result<()> {
        let _ = self.deliver_offline_for_uid(uid, client_id).await;
        Ok(())
    }
}
