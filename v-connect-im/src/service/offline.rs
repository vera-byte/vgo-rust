use anyhow::Result;
use crate::server::VConnectIMServer;

impl VConnectIMServer {
    pub async fn deliver_offline_for_uid(&self, uid: &str, client_id: &str) -> Result<usize> {
        let list = self.storage.pull_offline(uid, 1000)?;
        let mut delivered_ids: Vec<String> = Vec::new();
        for rec in list.iter() {
            let msg = crate::domain::message::ImMessage { msg_type: rec.msg_type.clone(), data: serde_json::json!({ "room_id": rec.room_id, "content": rec.content, "message_id": rec.message_id, "offline": true }), target_uid: None };
            let txt = serde_json::to_string(&msg)?;
            if self.send_message_to_client(client_id, tokio_tungstenite::tungstenite::Message::Text(txt)).await.is_ok() { delivered_ids.push(rec.message_id.clone()); }
        }
        let removed = self.storage.ack_offline(uid, &delivered_ids)?;
        Ok(removed)
    }

    pub async fn enforce_offline_quota_for_uid(&self, uid: &str) -> Result<usize> {
        let (max_count, cleanup_batch) = match v::get_global_config_manager() { Ok(cm) => ( cm.get_or("offline.max_per_uid", 500_i64) as usize, cm.get_or("offline.cleanup_batch", 50_i64) as usize ), Err(_) => (500usize, 50usize) };
        self.storage.enforce_offline_quota(uid, max_count, cleanup_batch)
    }
}

