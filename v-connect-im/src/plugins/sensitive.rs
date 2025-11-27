use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde_json::Value;

use crate::domain::message::ImMessage;
use crate::plugins::{Plugin, PluginContext, PluginFlow};

pub struct SensitiveWordPlugin {
    words: RwLock<Vec<String>>,
}

impl SensitiveWordPlugin {
    pub fn new(words: Vec<String>) -> Self {
        Self {
            words: RwLock::new(words),
        }
    }

    fn filter_value(&self, value: &mut Value) {
        if let Some(obj) = value.as_object_mut() {
            if let Some(content) = obj.get_mut("content") {
                if let Some(text) = content.as_str() {
                    let replaced = self.apply_filter(text);
                    *content = Value::String(replaced);
                }
            }
        }
    }

    fn apply_filter(&self, text: &str) -> String {
        let words = self.words.read();
        let mut output = text.to_string();
        for word in words.iter() {
            if !word.is_empty() && output.contains(word) {
                let replace = "*".repeat(word.len());
                output = output.replace(word, &replace);
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::ImMessage;

    #[tokio::test]
    async fn filter_replaces_words() {
        let plugin = SensitiveWordPlugin::new(vec!["bad".into()]);
        let server = crate::server::VConnectIMServer::new();
        let mut message = ImMessage {
            msg_type: "message".into(),
            data: serde_json::json!({"content":"bad text"}),
            target_uid: None,
        };
        let ctx = PluginContext::new(&server, "c1");
        plugin
            .on_message_incoming(&ctx, &mut message)
            .await
            .unwrap();
        assert_eq!(
            message
                .data
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap(),
            "*** text"
        );
    }
}

#[async_trait]
impl Plugin for SensitiveWordPlugin {
    fn name(&self) -> &'static str {
        "sensitive"
    }

    async fn on_message_incoming(
        &self,
        _ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        let mut cloned = message.data.clone();
        self.filter_value(&mut cloned);
        message.data = cloned;
        Ok(PluginFlow::Continue)
    }

    async fn on_message_outgoing(
        &self,
        _ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        let mut cloned = message.data.clone();
        self.filter_value(&mut cloned);
        message.data = cloned;
        Ok(PluginFlow::Continue)
    }

    async fn on_config_update(&self, config: &Value) -> Result<()> {
        if let Some(words) = config
            .get("plugins")
            .and_then(|v| v.get("sensitive_words"))
            .and_then(|v| v.as_array())
        {
            let new_words = words
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>();
            let mut guard = self.words.write();
            *guard = new_words;
        }
        Ok(())
    }
}
