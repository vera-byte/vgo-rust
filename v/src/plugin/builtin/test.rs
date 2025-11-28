use anyhow::Result;
use parking_lot::RwLock;
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::plugin::types::{ImMessageAccess, ImServer, PluginContextAccess};

pub struct TestPlugin {
    incoming_count: RwLock<usize>,
    outgoing_count: RwLock<usize>,
    custom_events: RwLock<Vec<(String, Value)>>,
    should_block: RwLock<bool>,
    test_data: RwLock<std::collections::HashMap<String, Value>>,
}

impl TestPlugin {
    pub fn new() -> Self {
        Self {
            incoming_count: RwLock::new(0),
            outgoing_count: RwLock::new(0),
            custom_events: RwLock::new(Vec::new()),
            should_block: RwLock::new(false),
            test_data: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn get_incoming_count(&self) -> usize { *self.incoming_count.read() }
    pub fn get_outgoing_count(&self) -> usize { *self.outgoing_count.read() }
    pub fn get_custom_events(&self) -> Vec<(String, Value)> { self.custom_events.read().clone() }
    pub fn set_should_block(&self, block: bool) { *self.should_block.write() = block; }
    pub fn get_test_data(&self, key: &str) -> Option<Value> { self.test_data.read().get(key).cloned() }
    pub fn set_test_data(&self, key: String, value: Value) { self.test_data.write().insert(key, value); }
    pub fn reset(&self) {
        *self.incoming_count.write() = 0;
        *self.outgoing_count.write() = 0;
        self.custom_events.write().clear();
        *self.should_block.write() = false;
        self.test_data.write().clear();
    }
    pub fn get_stats(&self) -> Value {
        serde_json::json!({
            "incoming_count": *self.incoming_count.read(),
            "outgoing_count": *self.outgoing_count.read(),
            "custom_events_count": self.custom_events.read().len(),
            "should_block": *self.should_block.read(),
            "test_data_keys": self.test_data.read().keys().cloned().collect::<Vec<_>>(),
        })
    }

    pub async fn on_message_incoming<S, C, M>(&self, ctx: &C, message: &mut M) -> Result<PluginFlow>
    where
        S: ImServer,
        C: PluginContextAccess<S>,
        M: ImMessageAccess,
    {
        *self.incoming_count.write() += 1;
        debug!(
            "TestPlugin: incoming message #{} from client {}: {:?}",
            *self.incoming_count.read(),
            ctx.client_id(),
            message.msg_type()
        );
        if *self.should_block.read() {
            warn!("TestPlugin: blocking message from client {}", ctx.client_id());
            return Ok(PluginFlow::Stop);
        }
        self.test_data.write().insert(
            format!("incoming_{}", *self.incoming_count.read()),
            serde_json::json!({
                "client_id": ctx.client_id(),
                "msg_type": message.msg_type(),
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(PluginFlow::Continue)
    }

    pub async fn on_message_outgoing<S, C, M>(&self, ctx: &C, message: &mut M) -> Result<PluginFlow>
    where
        S: ImServer,
        C: PluginContextAccess<S>,
        M: ImMessageAccess,
    {
        *self.outgoing_count.write() += 1;
        debug!(
            "TestPlugin: outgoing message #{} to client {}: {:?}",
            *self.outgoing_count.read(),
            ctx.client_id(),
            message.msg_type()
        );
        self.test_data.write().insert(
            format!("outgoing_{}", *self.outgoing_count.read()),
            serde_json::json!({
                "client_id": ctx.client_id(),
                "msg_type": message.msg_type(),
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(PluginFlow::Continue)
    }

    pub async fn on_startup<S: ImServer>(&self, server: &S) -> Result<()> {
        info!("TestPlugin: on_startup called");
        self.test_data.write().insert(
            "startup".to_string(),
            serde_json::json!({
                "node_id": server.node_id(),
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(())
    }

    pub async fn on_config_update(&self, config: &Value) -> Result<()> {
        info!("TestPlugin: on_config_update called");
        self.test_data.write().insert(
            "config_update".to_string(),
            serde_json::json!({
                "config": config,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(())
    }

    pub async fn on_shutdown(&self) -> Result<()> {
        info!("TestPlugin: on_shutdown called");
        self.test_data.write().insert(
            "shutdown".to_string(),
            serde_json::json!({
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(())
    }

    pub async fn on_custom_event(&self, event_type: &str, payload: &Value) -> Result<()> {
        debug!("TestPlugin: custom event received: {} - {:?}", event_type, payload);
        self.custom_events
            .write()
            .push((event_type.to_string(), payload.clone()));
        self.test_data.write().insert(
            format!("custom_{}", self.custom_events.read().len()),
            serde_json::json!({
                "event_type": event_type,
                "payload": payload,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        );
        Ok(())
    }
}

/// 流控制（与 v-connect-im 对齐）/ Flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFlow { Continue, Stop }
