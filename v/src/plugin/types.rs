use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// 消息访问接口 / Message access interface
pub trait ImMessageAccess {
    fn msg_type(&self) -> &str;
    fn set_msg_type(&mut self, s: String);
    fn data(&self) -> &Value;
    fn set_data(&mut self, v: Value);
    fn target_uid(&self) -> Option<&str>;
    fn set_target_uid(&mut self, uid: Option<String>);
}

/// 授权配置访问接口 / Auth config access interface
pub trait AuthConfigLiteAccess {
    fn enabled(&self) -> bool;
    fn center_url(&self) -> &str;
    fn timeout_ms(&self) -> u64;
}

/// 服务器抽象接口 / Server abstraction interface
#[async_trait]
pub trait ImServer: Send + Sync {
    fn node_id(&self) -> &str;
    fn auth_config(&self) -> Option<&dyn AuthConfigLiteAccess>;

    fn set_connection_uid(&self, client_id: &str, uid: &str);
    fn uid_clients_is_empty(&self, uid: &str) -> bool;
    fn uid_clients_insert(&self, uid: &str, client_id: &str);

    async fn emit_custom(&self, event_type: &str, payload: &Value) -> Result<()>;
    async fn deliver_offline_for_uid(&self, uid: &str, client_id: &str) -> Result<()>;
}

/// 插件上下文访问接口 / Plugin context access interface
pub trait PluginContextAccess<S: ImServer> {
    fn client_id(&self) -> &str;
    fn server(&self) -> Option<&S>;
}
