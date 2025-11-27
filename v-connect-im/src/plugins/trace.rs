use crate::domain::message::ImMessage;
use crate::plugins::{Plugin, PluginContext, PluginFlow};
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

/// 简易日志插件，演示如何截获上下行消息 / Trace plugin demo
pub struct TracePlugin {
    log_payload: bool,
}

impl TracePlugin {
    pub fn new(log_payload: bool) -> Self {
        Self { log_payload }
    }

    fn log_message(&self, direction: &str, ctx: &PluginContext<'_>, msg: &ImMessage) {
        if self.log_payload {
            debug!(
                "[plugin:trace] {} client={} type={} payload={}",
                direction, ctx.client_id, msg.msg_type, msg.data
            );
        } else {
            debug!(
                "[plugin:trace] {} client={} type={}",
                direction, ctx.client_id, msg.msg_type
            );
        }
    }
}

#[async_trait]
impl Plugin for TracePlugin {
    fn name(&self) -> &'static str {
        "trace"
    }

    fn priority(&self) -> u8 {
        1
    }

    async fn on_message_incoming(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        self.log_message("incoming", ctx, message);
        Ok(PluginFlow::Continue)
    }

    async fn on_message_outgoing(
        &self,
        ctx: &PluginContext<'_>,
        message: &mut ImMessage,
    ) -> Result<PluginFlow> {
        self.log_message("outgoing", ctx, message);
        Ok(PluginFlow::Continue)
    }
}
