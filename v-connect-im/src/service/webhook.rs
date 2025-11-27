use crate::server::VConnectIMServer;
use crate::config::WebhookConfigLite as WebhookConfig;
use crate::domain::message::{
    WebhookEvent,
    WebhookEventType,
    WebhookClientStatusData,
    WebhookMessageData,
};
use anyhow::Result;
use tracing::{error, info, warn};

// 发送Webhook事件 / Send Webhook Event
pub async fn send_webhook_event(server: &VConnectIMServer, event_type: WebhookEventType, data: serde_json::Value) {
    let event_key = format!("webhook.{}", format!("{:?}", event_type).to_lowercase());
    if let Err(e) = server
        .plugin_registry
        .emit_custom(&event_key, &data)
        .await
    {
        warn!("plugin custom event error: {}", e);
    }
    if let Some(webhook_config) = &server.webhook_config {
        if !webhook_config.enabled { return; }
        let event = WebhookEvent {
            event_type: event_type.clone(),
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            data,
            retry_count: Some(0),
        };
        let webhook_config = webhook_config.clone();
        tokio::spawn(async move {
            if let Err(e) = deliver_webhook_event(webhook_config, event).await {
                error!("❌ Failed to deliver webhook event: {}", e);
            }
        });
    }
}

// 交付Webhook事件到第三方服务器 / Deliver Webhook Event to Third-party Server
pub async fn deliver_webhook_event(webhook_config: WebhookConfig, event: WebhookEvent) -> Result<()> {
    if webhook_config.url.is_none() { return Ok(()); }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(webhook_config.timeout_ms))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

    let mut request = client.post(webhook_config.url.as_ref().unwrap()).json(&event);
    if let Some(secret) = &webhook_config.secret {
        let signature = generate_webhook_signature(&event, secret);
        request = request.header("X-VConnectIM-Signature", signature);
    }

    let response = request.send().await.map_err(|e| anyhow::anyhow!("Webhook request failed: {}", e))?;
    if response.status().is_success() {
        info!("✅ Webhook event {} delivered successfully", event.event_id);
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(anyhow::anyhow!("Webhook delivery failed with status {}: {}", status, body))
    }
}

// 生成Webhook签名 / Generate Webhook Signature
pub fn generate_webhook_signature(event: &WebhookEvent, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let payload = serde_json::json!({
        "event_id": event.event_id,
        "event_type": format!("{:?}", event.event_type),
        "timestamp": event.timestamp,
    })
    .to_string();
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    format!("sha256={}", hex::encode(code_bytes))
}

// 发送客户端上线Webhook事件 / Send Client Online Webhook Event
pub async fn send_client_online_webhook(server: &VConnectIMServer, client_id: &str, uid: &Option<String>, addr: &std::net::SocketAddr) {
    let data = serde_json::json!(WebhookClientStatusData {
        client_id: client_id.to_string(),
        uid: uid.clone(),
        addr: addr.to_string(),
        connected_at: Some(chrono::Utc::now().timestamp_millis()),
        disconnected_at: None,
        online_duration_ms: None,
    });
    send_webhook_event(server, WebhookEventType::ClientOnline, data).await;
}

// 发送客户端离线Webhook事件 / Send Client Offline Webhook Event
pub async fn send_client_offline_webhook(server: &VConnectIMServer, client_id: &str, uid: &Option<String>, addr: &std::net::SocketAddr, connected_at: i64) {
    let now = chrono::Utc::now().timestamp_millis();
    let online_duration_ms = (now - connected_at).max(0) as u64;
    let data = serde_json::json!(WebhookClientStatusData {
        client_id: client_id.to_string(),
        uid: uid.clone(),
        addr: addr.to_string(),
        connected_at: Some(connected_at),
        disconnected_at: Some(now),
        online_duration_ms: Some(online_duration_ms),
    });
    send_webhook_event(server, WebhookEventType::ClientOffline, data).await;
}

// 发送消息Webhook事件 / Send Message Webhook Event
pub async fn send_message_webhook(
    server: &VConnectIMServer,
    message_id: &str,
    from_client_id: &str,
    from_uid: &Option<String>,
    to_client_id: &Option<String>,
    to_uid: &Option<String>,
    content: &serde_json::Value,
    message_type: &str,
    delivery_status: &str,
    delivered_at: Option<i64>,
) {
    let data = serde_json::json!(WebhookMessageData {
        message_id: message_id.to_string(),
        from_client_id: from_client_id.to_string(),
        from_uid: from_uid.clone(),
        to_client_id: to_client_id.clone(),
        to_uid: to_uid.clone(),
        content: content.clone(),
        message_type: message_type.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        delivered_at,
        delivery_status: delivery_status.to_string(),
    });
    let event_type = match delivery_status { "delivered" => WebhookEventType::MessageDelivered, "failed" => WebhookEventType::MessageFailed, _ => WebhookEventType::MessageSent };
    send_webhook_event(server, event_type, data).await;
}
