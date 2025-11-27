use anyhow::Result;
use crate::server::VConnectIMServer;

/// 验证令牌 / Validate token
pub async fn validate_token(server: &VConnectIMServer, token: &str) -> Result<bool> {
    if token.is_empty() { return Ok(false); }
    if let Some(cfg) = &server.auth_config {
        if !cfg.enabled { return Ok(true); }
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_millis(cfg.timeout_ms)).build()?;
        let resp = client.get(format!("{}/v1/sso/auth", cfg.center_url)).query(&[("token", token)]).send().await?;
        Ok(resp.status().is_success())
    } else { Ok(true) }
}

/// 应用鉴权结果并建立UID映射 / Apply auth and build UID mapping
pub async fn apply_auth(server: &VConnectIMServer, client_id: &str, uid: &str) -> Result<()> {
    if let Some(mut conn) = server.connections.get_mut(client_id) { conn.uid = Some(uid.to_string()); }
    let set = server.uid_clients.entry(uid.to_string()).or_default();
    set.insert(client_id.to_string());
    let _ = server.deliver_offline_for_uid(uid, client_id).await;
    Ok(())
}

