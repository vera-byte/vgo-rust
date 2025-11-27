use anyhow::Result;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::server::VConnectIMServer;
use crate::domain::message::{HttpSendMessageRequest, HttpSendMessageResponse, ImMessage};
use crate::storage;

impl VConnectIMServer {
    pub async fn http_send_message(&self, request: HttpSendMessageRequest) -> HttpSendMessageResponse {
        let message_id = Uuid::new_v4().to_string();
        let delivered_at = chrono::Utc::now().timestamp_millis();
        let message_type = request.message_type.clone().unwrap_or_else(|| "message".to_string());
        let forward_msg = ImMessage { msg_type: "forwarded_message".to_string(), data: serde_json::json!({"from": request.from_uid, "content": request.content, "timestamp": delivered_at, "message_id": message_id}), target_uid: None };
        let forward_json = serde_json::to_string(&forward_msg).unwrap_or_default();
        let record = storage::MessageRecord { message_id: message_id.clone(), from_client_id: request.from_uid.clone(), to_client_id: request.to_uid.clone(), content: request.content.clone(), timestamp: delivered_at, msg_type: message_type.clone(), room_id: None };
        let _ = self.raft.append_entry_as(&self.node_id, &record);
        let _ = self.storage.append(&record);
        let mut ok = false;
        if let Some(clients) = self.uid_clients.get(&request.to_uid) { for cid in clients.iter() { if self.send_message_to_client(&cid, Message::Text(forward_json.clone())).await.is_ok() { ok = true; } } }
        if !ok { self.await_ack_or_queue_offline(request.to_uid.clone(), message_id.clone(), None, request.content.clone(), message_type.clone(), v::get_global_config_manager().ok().map(|cm| cm.get_or("delivery.ack_deadline_ms", 1000_u64)).unwrap_or(1000)).await; }
        HttpSendMessageResponse { success: true, message: "ok".to_string(), message_id: Some(message_id), delivered_at: Some(delivered_at) }
    }

    pub async fn await_ack_or_queue_offline(&self, recipient_uid: String, message_id: String, room_id: Option<String>, content: serde_json::Value, msg_type: String, deadline_ms: u64) {
        let server = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(deadline_ms)).await;
            let acked = server.acked_ids.get(&recipient_uid).map(|set| set.contains(&message_id)).unwrap_or(false);
            if !acked {
                let _ = server.enforce_offline_quota_for_uid(&recipient_uid).await;
                let off = storage::OfflineRecord { message_id: message_id.clone(), from_uid: None, to_uid: recipient_uid.clone(), room_id: room_id.clone(), content: content.clone(), timestamp: chrono::Utc::now().timestamp_millis(), msg_type: msg_type.clone() };
                let _ = server.storage.store_offline(&off);
                server.send_message_webhook(&message_id, &recipient_uid, &None, &None, &Some(recipient_uid.clone()), &content, &msg_type, "queued_offline", None).await;
            }
        });
    }
}
