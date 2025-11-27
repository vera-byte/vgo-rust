//! 插件系统入口 / Plugin system entry

pub mod auth;
pub mod bridge;
pub mod sensitive;
pub mod trace;

use crate::domain::message::ImMessage;
use crate::server::VConnectIMServer;
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::Value;
use std::sync::Arc;

/// 插件执行流程控制 / Flow control returned by plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFlow {
    Continue,
    Stop,
}

/// 插件上下文信息 / Context passed to plugins
pub struct PluginContext<'a> {
    pub server: Option<&'a VConnectIMServer>,
    pub client_id: &'a str,
}

impl<'a> PluginContext<'a> {
    pub fn new(server: &'a VConnectIMServer, client_id: &'a str) -> Self {
        Self {
            server: Some(server),
            client_id,
        }
    }

    pub fn system(client_id: &'a str) -> Self {
        Self {
            server: None,
            client_id,
        }
    }
}

/// 插件公共trait / Common trait for plugins
#[async_trait]
pub trait Plugin: Send + Sync {
    /// 插件唯一名称 / Plugin identifier
    fn name(&self) -> &'static str;

    /// 插件优先级（越小越先执行） / Order of execution
    fn priority(&self) -> u8 {
        100
    }

    /// 处理上行消息（客户端->服务器） / Incoming hook
    async fn on_message_incoming(
        &self,
        _ctx: &PluginContext<'_>,
        _message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        Ok(PluginFlow::Continue)
    }

    /// 处理下行消息（服务器->客户端） / Outgoing hook
    async fn on_message_outgoing(
        &self,
        _ctx: &PluginContext<'_>,
        _message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        Ok(PluginFlow::Continue)
    }

    /// 插件启动时机 / Called when plugin system starts
    async fn on_startup(&self, _server: &VConnectIMServer) -> Result<()> {
        Ok(())
    }

    /// 配置更新 / Called when configuration updates
    async fn on_config_update(&self, _config: &Value) -> Result<()> {
        Ok(())
    }

    /// 插件关闭钩子 / Called before shutdown
    async fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// 自定义事件钩子 / Custom event hook
    async fn on_custom_event(&self, _event_type: &str, _payload: &Value) -> Result<()> {
        Ok(())
    }
}

/// 插件注册中心 / Plugin registry
#[derive(Default)]
pub struct PluginRegistry {
    plugins: RwLock<Vec<Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
        }
    }

    /// 注册插件 / Register plugin
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        let mut guard = self.plugins.write();
        guard.push(plugin);
        guard.sort_by_key(|p| p.priority());
    }

    fn snapshot(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.read().clone()
    }

    /// 触发上行钩子 / Emit incoming hooks
    pub async fn emit_incoming(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        for plugin in self.snapshot() {
            match plugin.on_message_incoming(ctx, message).await? {
                PluginFlow::Continue => continue,
                PluginFlow::Stop => return Ok(PluginFlow::Stop),
            }
        }
        Ok(PluginFlow::Continue)
    }

    /// 触发下行钩子 / Emit outgoing hooks
    pub async fn emit_outgoing(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        for plugin in self.snapshot() {
            match plugin.on_message_outgoing(ctx, message).await? {
                PluginFlow::Continue => continue,
                PluginFlow::Stop => return Ok(PluginFlow::Stop),
            }
        }
        Ok(PluginFlow::Continue)
    }
}

impl PluginRegistry {
    pub async fn emit_startup(&self, server: &VConnectIMServer) -> Result<()> {
        for plugin in self.snapshot() {
            plugin.on_startup(server).await?;
        }
        Ok(())
    }

    pub async fn emit_config_update(&self, config: &Value) -> Result<()> {
        for plugin in self.snapshot() {
            plugin.on_config_update(config).await?;
        }
        Ok(())
    }

    pub async fn emit_shutdown(&self) -> Result<()> {
        for plugin in self.snapshot() {
            plugin.on_shutdown().await?;
        }
        Ok(())
    }

    pub async fn emit_custom(&self, event_type: &str, payload: &Value) -> Result<()> {
        for plugin in self.snapshot() {
            plugin.on_custom_event(event_type, payload).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::VConnectIMServer;
    use serde_json::json;

    struct BlockPlugin;

    #[async_trait]
    impl Plugin for BlockPlugin {
        fn name(&self) -> &'static str {
            "block"
        }

        async fn on_message_incoming(
            &self,
            _ctx: &PluginContext<'_>,
            message: &mut ImMessage,
        ) -> Result<PluginFlow> {
            message.msg_type = "blocked".into();
            Ok(PluginFlow::Stop)
        }
    }

    #[tokio::test]
    async fn plugin_registry_stops_flow() {
        let registry = PluginRegistry::new();
        registry.register(Arc::new(BlockPlugin));
        let server = VConnectIMServer::new();
        let ctx = PluginContext::new(&server, "c1");
        let mut message = ImMessage {
            msg_type: "message".into(),
            data: json!({"text":"hello"}),
            target_uid: None,
        };
        let res = registry.emit_incoming(&ctx, &mut message).await.unwrap();
        assert_eq!(res, PluginFlow::Stop);
        assert_eq!(message.msg_type, "blocked");
    }
}
