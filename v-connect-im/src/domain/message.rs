use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// IM 消息结构 / IM Message Structure
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ImMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_uid: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ConnectRequest {
    pub uid: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ConnectResponse {
    pub status: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct OnlineClientInfo {
    pub uid: Option<String>,
    pub addr: String,
    pub connected_at: i64,
    pub last_heartbeat: i64,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct OnlineClientsResponse {
    pub clients: Vec<OnlineClientInfo>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct HttpSendMessageRequest {
    pub from_uid: String,
    pub to_uid: String,
    pub content: serde_json::Value,
    pub message_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct HttpSendMessageResponse {
    pub success: bool,
    pub message: String,
    pub message_id: Option<String>,
    pub delivered_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct HttpBroadcastRequest {
    pub from_uid: String,
    pub content: serde_json::Value,
    pub message_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct HttpBroadcastResponse {
    pub success: bool,
    pub message: String,
    pub delivered_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    ClientOnline,
    ClientOffline,
    MessageSent,
    MessageDelivered,
    MessageFailed,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct WebhookEvent {
    pub event_type: WebhookEventType,
    pub event_id: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
    pub retry_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct WebhookClientStatusData {
    pub client_id: String,
    pub uid: Option<String>,
    pub addr: String,
    pub connected_at: Option<i64>,
    pub disconnected_at: Option<i64>,
    pub online_duration_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct WebhookMessageData {
    pub message_id: String,
    pub from_client_id: String,
    pub from_uid: Option<String>,
    pub to_client_id: Option<String>,
    pub to_uid: Option<String>,
    pub content: serde_json::Value,
    pub message_type: String,
    pub timestamp: i64,
    pub delivered_at: Option<i64>,
    pub delivery_status: String,
}
