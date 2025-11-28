use anyhow::Result;
use tracing::debug;

use crate::plugin::types::{ImMessageAccess, ImServer, PluginContextAccess};

/// 简易日志插件 / Simple trace plugin
pub struct TracePlugin {
    log_payload: bool,
}

impl TracePlugin {
    pub fn new(log_payload: bool) -> Self { Self { log_payload } }

    fn log_message<S, C, M>(&self, direction: &str, ctx: &C, msg: &M)
    where
        S: ImServer,
        C: PluginContextAccess<S>,
        M: ImMessageAccess,
    {
        if self.log_payload {
            debug!(
                "[plugin:trace] {} client={} type={} payload={}",
                direction,
                ctx.client_id(),
                msg.msg_type(),
                msg.data()
            );
        } else {
            debug!(
                "[plugin:trace] {} client={} type={}",
                direction,
                ctx.client_id(),
                msg.msg_type()
            );
        }
    }

    pub async fn on_message_incoming<S, C, M>(&self, ctx: &C, message: &mut M) -> Result<PluginFlow>
    where
        S: ImServer,
        C: PluginContextAccess<S>,
        M: ImMessageAccess,
    {
        self.log_message::<S, C, M>("incoming", ctx, message);
        Ok(PluginFlow::Continue)
    }

    pub async fn on_message_outgoing<S, C, M>(&self, ctx: &C, message: &mut M) -> Result<PluginFlow>
    where
        S: ImServer,
        C: PluginContextAccess<S>,
        M: ImMessageAccess,
    {
        self.log_message::<S, C, M>("outgoing", ctx, message);
        Ok(PluginFlow::Continue)
    }
}

/// 流控制（与 v-connect-im 对齐）/ Flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFlow { Continue, Stop }
