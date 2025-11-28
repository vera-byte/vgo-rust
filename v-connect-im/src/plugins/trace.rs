use crate::domain::message::ImMessage;
use crate::plugins::{Plugin, PluginContext, PluginFlow};
use anyhow::Result;
use async_trait::async_trait;
use v::plugin::builtin::trace::{PluginFlow as VFlow, TracePlugin as VTracePlugin};

pub struct TracePlugin {
    inner: VTracePlugin,
}

impl TracePlugin {
    pub fn new(log_payload: bool) -> Self { Self { inner: VTracePlugin::new(log_payload) } }
}

#[async_trait]
impl Plugin for TracePlugin {
    fn name(&self) -> &'static str { "trace" }
    fn priority(&self) -> u8 { 1 }
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
}
