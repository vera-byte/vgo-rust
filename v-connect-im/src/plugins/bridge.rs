use anyhow::{anyhow, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error};

use crate::domain::message::ImMessage;
use crate::plugins::{Plugin, PluginContext, PluginFlow};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RemotePluginSummary {
    pub plugin_id: String,
    pub name: String,
    pub callback_url: String,
    pub capabilities: Vec<String>,
    pub last_heartbeat_ts: i64,
}

#[derive(Clone)]
pub struct RemotePluginInfo {
    pub plugin_id: String,
    pub name: String,
    pub callback_url: String,
    pub token: String,
    pub capabilities: Vec<String>,
    pub last_heartbeat: Instant,
    pub last_heartbeat_ts: i64,
}

impl RemotePluginInfo {
    pub fn summary(&self) -> RemotePluginSummary {
        RemotePluginSummary {
            plugin_id: self.plugin_id.clone(),
            name: self.name.clone(),
            callback_url: self.callback_url.clone(),
            capabilities: self.capabilities.clone(),
            last_heartbeat_ts: self.last_heartbeat_ts,
        }
    }
}

#[derive(Clone)]
pub struct PendingEvent {
    pub plugin_id: String,
    pub event_type: String,
    pub created_at: Instant,
}

#[derive(Default)]
pub struct RemotePluginManager {
    entries: DashMap<String, RemotePluginInfo>,
    pending_events: DashMap<String, PendingEvent>,
}

impl RemotePluginManager {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            pending_events: DashMap::new(),
        }
    }

    /// 注册新插件 / Register a new plugin
    pub fn register(
        &self,
        name: String,
        callback_url: String,
        capabilities: Vec<String>,
    ) -> RemotePluginInfo {
        let plugin_id = format!("plg_{}", uuid::Uuid::new_v4().to_string());
        let token: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let info = RemotePluginInfo {
            plugin_id: plugin_id.clone(),
            name,
            callback_url,
            token: token.clone(),
            capabilities,
            last_heartbeat: Instant::now(),
            last_heartbeat_ts: chrono::Utc::now().timestamp_millis(),
        };
        self.entries.insert(plugin_id.clone(), info.clone());
        info
    }

    /// 插件重连（使用已有 plugin_id + token 更新 callback_url）
    /// Reconnect plugin (update callback_url using existing plugin_id + token)
    pub fn reconnect(
        &self,
        plugin_id: &str,
        token: &str,
        callback_url: String,
        capabilities: Option<Vec<String>>,
    ) -> Result<RemotePluginInfo> {
        if let Some(mut entry) = self.entries.get_mut(plugin_id) {
            if entry.token != token {
                return Err(anyhow!("invalid token"));
            }
            entry.callback_url = callback_url;
            if let Some(caps) = capabilities {
                entry.capabilities = caps;
            }
            entry.last_heartbeat = Instant::now();
            entry.last_heartbeat_ts = chrono::Utc::now().timestamp_millis();
            Ok(entry.clone())
        } else {
            Err(anyhow!("plugin not found"))
        }
    }

    pub fn heartbeat(&self, plugin_id: &str, token: &str) -> Result<()> {
        if let Some(mut entry) = self.entries.get_mut(plugin_id) {
            if entry.token != token {
                return Err(anyhow!("invalid token"));
            }
            entry.last_heartbeat = Instant::now();
            entry.last_heartbeat_ts = chrono::Utc::now().timestamp_millis();
            Ok(())
        } else {
            Err(anyhow!("plugin not found"))
        }
    }

    pub fn list(&self) -> Vec<RemotePluginSummary> {
        self.entries
            .iter()
            .map(|entry| entry.value().summary())
            .collect()
    }

    pub fn remove_inactive(&self, ttl: Duration) {
        let now = Instant::now();
        let keys: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| now.duration_since(entry.last_heartbeat) > ttl)
            .map(|entry| entry.plugin_id.clone())
            .collect();
        for key in keys {
            self.entries.remove(&key);
        }
    }

    pub fn snapshot(&self) -> Vec<RemotePluginInfo> {
        self.entries
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn validate(&self, plugin_id: &str, token: &str) -> Result<()> {
        if let Some(entry) = self.entries.get(plugin_id) {
            if entry.token == token {
                Ok(())
            } else {
                Err(anyhow!("invalid token"))
            }
        } else {
            Err(anyhow!("plugin not found"))
        }
    }

    pub fn unregister(&self, plugin_id: &str, token: &str) -> Result<()> {
        self.validate(plugin_id, token)?;
        self.entries.remove(plugin_id);
        let keys: Vec<String> = self
            .pending_events
            .iter()
            .filter(|entry| entry.plugin_id == plugin_id)
            .map(|entry| entry.key().clone())
            .collect();
        for key in keys {
            self.pending_events.remove(&key);
        }
        Ok(())
    }

    pub fn record_event(&self, plugin_id: &str, event_type: &str) -> String {
        let event_id = uuid::Uuid::new_v4().to_string();
        self.pending_events.insert(
            event_id.clone(),
            PendingEvent {
                plugin_id: plugin_id.to_string(),
                event_type: event_type.to_string(),
                created_at: Instant::now(),
            },
        );
        event_id
    }

    pub fn ack_event(&self, plugin_id: &str, token: &str, event_id: &str) -> Result<String> {
        self.validate(plugin_id, token)?;
        match self.pending_events.remove(event_id) {
            Some((_id, pending)) => {
                if pending.plugin_id != plugin_id {
                    return Err(anyhow!("event not owned by plugin"));
                }
                Ok(pending.event_type)
            }
            None => Err(anyhow!("event not found")),
        }
    }

    pub fn cleanup_pending(&self, ttl: Duration) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .pending_events
            .iter()
            .filter(|entry| now.duration_since(entry.created_at) > ttl)
            .map(|entry| entry.key().clone())
            .collect();
        for key in expired {
            self.pending_events.remove(&key);
        }
    }
}

pub struct HttpBridgePlugin {
    manager: Arc<RemotePluginManager>,
    client: Client,
    pending_ttl: Duration,
    last_config: RwLock<Option<Value>>,
}

impl HttpBridgePlugin {
    pub fn new(manager: Arc<RemotePluginManager>, timeout_ms: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        Ok(Self {
            manager,
            client,
            pending_ttl: Duration::from_secs(300),
            last_config: RwLock::new(None),
        })
    }

    /// 派发事件到所有远程插件 / Dispatch event to all remote plugins
    async fn dispatch(
        &self,
        direction: &str,
        event_type: &str,
        ctx: &PluginContext<'_>,
        payload: Value,
        require_ack: bool,
    ) -> Result<PluginFlow> {
        let snapshot = self.manager.snapshot();
        if snapshot.is_empty() {
            return Ok(PluginFlow::Continue);
        }
        debug!(
            "📤 Dispatching event: type={} direction={} client={} to {} plugin(s)",
            event_type,
            direction,
            ctx.client_id,
            snapshot.len()
        );
        for plugin in snapshot {
            let event_id = if require_ack {
                Some(self.manager.record_event(&plugin.plugin_id, event_type))
            } else {
                None
            };
            let mut body = payload.clone();
            body["event_type"] = Value::String(event_type.to_string());
            body["direction"] = Value::String(direction.to_string());
            body["client_id"] = Value::String(ctx.client_id.to_string());
            body["timestamp"] = json!(chrono::Utc::now().timestamp_millis());
            if let Some(id) = &event_id {
                body["event_id"] = Value::String(id.clone());
            }
            let request = self
                .client
                .post(&plugin.callback_url)
                .header("X-VIM-Plugin-ID", &plugin.plugin_id)
                .header("X-VIM-Plugin-Token", &plugin.token)
                .json(&body);
            let name = plugin.name.clone();
            let callback = plugin.callback_url.clone();
            let plugin_id = plugin.plugin_id.clone();
            let fut = request.send();
            let res = fut.await;
            match res {
                Ok(resp) => {
                    if resp.status().is_success() {
                        debug!(
                            "📨 Event dispatched: plugin={} type={} event_id={:?}",
                            plugin_id, event_type, event_id
                        );
                    } else {
                        error!(
                            "plugin {} callback error status={} url={}",
                            name,
                            resp.status(),
                            callback
                        );
                    }
                }
                Err(e) => {
                    error!("plugin {} callback error: {}", name, e);
                }
            }
        }
        Ok(PluginFlow::Continue)
    }

    async fn cleanup_loop(manager: Arc<RemotePluginManager>, ttl: Duration) {
        loop {
            sleep(Duration::from_secs(30)).await;
            manager.remove_inactive(ttl);
            manager.cleanup_pending(ttl);
        }
    }
}

#[async_trait]
impl Plugin for HttpBridgePlugin {
    fn name(&self) -> &'static str {
        "http_bridge"
    }

    fn priority(&self) -> u8 {
        10
    }

    async fn on_message_incoming(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        let payload = json!({
            "message": message,
            "category": "message",
        });
        self.dispatch("incoming", "message.incoming", ctx, payload, true)
            .await
    }

    async fn on_message_outgoing(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        let payload = json!({
            "message": message,
            "category": "message",
        });
        self.dispatch("outgoing", "message.outgoing", ctx, payload, true)
            .await
    }

    async fn on_startup(&self, _server: &crate::server::VConnectIMServer) -> Result<()> {
        let manager = self.manager.clone();
        let ttl = self.pending_ttl;
        tokio::spawn(async move {
            HttpBridgePlugin::cleanup_loop(manager, ttl).await;
        });
        Ok(())
    }

    async fn on_config_update(&self, config: &Value) -> Result<()> {
        {
            let mut guard = self.last_config.write();
            *guard = Some(config.clone());
        }
        let payload = json!({
            "category": "control",
            "config": config,
        });
        let ctx = PluginContext::system("system");
        let _ = self
            .dispatch("control", "control.config_update", &ctx, payload, false)
            .await;
        Ok(())
    }

    async fn on_shutdown(&self) -> Result<()> {
        let payload = json!({
            "category": "control",
            "state": "stopping",
        });
        let ctx = PluginContext::system("system");
        let _ = self
            .dispatch("control", "control.stop", &ctx, payload, false)
            .await;
        Ok(())
    }

    async fn on_custom_event(&self, event_type: &str, payload: &Value) -> Result<()> {
        let ctx = PluginContext::system("system");
        let require_ack = event_type.starts_with("message.");
        let _ = self
            .dispatch("custom", event_type, &ctx, payload.clone(), require_ack)
            .await?;
        Ok(())
    }
}
