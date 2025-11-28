//! 插件测试模块 / Plugin test module
//!
//! 提供测试插件和测试工具，用于验证插件系统的各项功能
//! Provides test plugin and test tools to verify plugin system functionality

use crate::domain::message::ImMessage;
use crate::plugins::{Plugin, PluginContext, PluginFlow};
use crate::server::VConnectIMServer;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use v::plugin::builtin::test::{PluginFlow as VFlow, TestPlugin as VTestPlugin};

/// 测试插件 / Test plugin
/// 用于验证插件系统的各项功能
/// Used to verify plugin system functionality
pub struct TestPlugin { inner: Arc<VTestPlugin> }

impl TestPlugin {
    /// 创建新的测试插件 / Create new test plugin
    pub fn new() -> Self {
        Self { inner: Arc::new(VTestPlugin::new()) }
    }

    /// 获取接收消息计数 / Get incoming message count
    pub fn get_incoming_count(&self) -> usize { self.inner.get_incoming_count() }

    /// 获取发送消息计数 / Get outgoing message count
    pub fn get_outgoing_count(&self) -> usize { self.inner.get_outgoing_count() }

    /// 获取自定义事件列表 / Get custom events list
    pub fn get_custom_events(&self) -> Vec<(String, Value)> { self.inner.get_custom_events() }

    /// 设置是否阻止消息 / Set whether to block messages
    pub fn set_should_block(&self, block: bool) { self.inner.set_should_block(block) }

    /// 获取测试数据 / Get test data
    pub fn get_test_data(&self, key: &str) -> Option<Value> { self.inner.get_test_data(key) }

    /// 设置测试数据 / Set test data
    pub fn set_test_data(&self, key: String, value: Value) { self.inner.set_test_data(key, value) }

    /// 重置所有计数器和数据 / Reset all counters and data
    pub fn reset(&self) { self.inner.reset() }

    /// 获取统计信息 / Get statistics
    pub fn get_stats(&self) -> Value { self.inner.get_stats() }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn name(&self) -> &'static str {
        "test"
    }

    fn priority(&self) -> u8 {
        1 // 高优先级，优先执行 / High priority, execute first
    }

    async fn on_message_incoming(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        match self.inner.on_message_incoming::<crate::server::VConnectIMServer, _, _>(ctx, message).await? {
            VFlow::Continue => Ok(PluginFlow::Continue),
            VFlow::Stop => Ok(PluginFlow::Stop),
        }
    }

    async fn on_message_outgoing(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        match self.inner.on_message_outgoing::<crate::server::VConnectIMServer, _, _>(ctx, message).await? {
            VFlow::Continue => Ok(PluginFlow::Continue),
            VFlow::Stop => Ok(PluginFlow::Stop),
        }
    }

    async fn on_startup(&self, server: &VConnectIMServer) -> Result<()> { self.inner.on_startup(server).await }

    async fn on_config_update(&self, config: &Value) -> Result<()> { self.inner.on_config_update(config).await }

    async fn on_shutdown(&self) -> Result<()> { self.inner.on_shutdown().await }

    async fn on_custom_event(&self, event_type: &str, payload: &Value) -> Result<()> { self.inner.on_custom_event(event_type, payload).await }
}

/// 测试插件管理器 / Test plugin manager
/// 用于管理和访问测试插件实例
/// Used to manage and access test plugin instances
pub struct TestPluginManager {
    test_plugin: Arc<TestPlugin>,
}

impl TestPluginManager {
    /// 创建新的测试插件管理器 / Create new test plugin manager
    pub fn new() -> Self {
        Self {
            test_plugin: Arc::new(TestPlugin::new()),
        }
    }

    /// 获取测试插件实例 / Get test plugin instance
    pub fn get_plugin(&self) -> Arc<TestPlugin> {
        self.test_plugin.clone()
    }
}

impl Default for TestPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::VConnectIMServer;

    #[tokio::test]
    async fn test_plugin_counters() {
        let plugin = TestPlugin::new();
        assert_eq!(plugin.get_incoming_count(), 0);
        assert_eq!(plugin.get_outgoing_count(), 0);

        let server = VConnectIMServer::new();
        let ctx = PluginContext::new(&server, "test_client");
        let mut msg = ImMessage {
            msg_type: "test".to_string(),
            data: serde_json::json!({}),
            target_uid: None,
        };

        let _ = plugin.on_message_incoming(&ctx, &mut msg).await;
        assert_eq!(plugin.get_incoming_count(), 1);

        let _ = plugin.on_message_outgoing(&ctx, &mut msg).await;
        assert_eq!(plugin.get_outgoing_count(), 1);
    }

    #[tokio::test]
    async fn test_plugin_block() {
        let plugin = TestPlugin::new();
        plugin.set_should_block(true);

        let server = VConnectIMServer::new();
        let ctx = PluginContext::new(&server, "test_client");
        let mut msg = ImMessage {
            msg_type: "test".to_string(),
            data: serde_json::json!({}),
            target_uid: None,
        };

        let flow = plugin.on_message_incoming(&ctx, &mut msg).await.unwrap();
        assert_eq!(flow, PluginFlow::Stop);
    }

    #[tokio::test]
    async fn test_plugin_custom_events() {
        let plugin = TestPlugin::new();
        let payload = serde_json::json!({"test": "data"});

        let _ = plugin.on_custom_event("test.event", &payload).await;
        let events = plugin.get_custom_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "test.event");
    }
}
