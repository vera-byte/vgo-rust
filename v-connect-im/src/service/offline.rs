use crate::server::VConnectIMServer;
use anyhow::Result;

impl VConnectIMServer {
    pub async fn deliver_offline_for_uid(&self, _uid: &str, _client_id: &str) -> Result<usize> {
        // 离线消息投递已迁移到插件 / Offline delivery migrated to plugin
        // TODO: 使用插件实现 / Use plugin implementation
        Ok(0)
    }

    pub async fn enforce_offline_quota_for_uid(&self, _uid: &str) -> Result<usize> {
        // 离线消息配额管理已迁移到插件 / Offline quota migrated to plugin
        // TODO: 使用插件实现 / Use plugin implementation
        Ok(0)
    }
}
