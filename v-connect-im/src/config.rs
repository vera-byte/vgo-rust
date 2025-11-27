use anyhow::Result;

#[derive(Clone)]
pub struct ServerConfig { pub host: String, pub ws_port: u16, pub http_port: u16, pub timeout_ms: u64, pub node_id: String }
#[derive(Clone)]
pub struct AuthConfigLite { pub enabled: bool, pub center_url: String, pub timeout_ms: u64 }
#[derive(Clone)]
pub struct WebhookConfigLite { pub url: Option<String>, pub timeout_ms: u64, pub secret: Option<String>, pub enabled: bool }

pub fn load() -> Result<(ServerConfig, AuthConfigLite, WebhookConfigLite)> {
    let cm = v::get_global_config_manager()?;
    Ok((
        ServerConfig { host: cm.get_or("server.host", "127.0.0.1".to_string()), ws_port: cm.get_or("server.ws_port", 5200_i64) as u16, http_port: cm.get_or("server.http_port", 8080_i64) as u16, timeout_ms: cm.get_or("server.timeout_ms", 10000_i64) as u64, node_id: cm.get_or("server.node_id", "node-local".to_string()) },
        AuthConfigLite { enabled: cm.get_or("auth.enabled", false), center_url: cm.get_or("auth.center_url", "http://127.0.0.1:8090".to_string()), timeout_ms: cm.get_or("auth.timeout_ms", 1000_i64) as u64 },
        WebhookConfigLite { url: cm.get::<String>("webhook.url").ok(), timeout_ms: cm.get_or("webhook.timeout_ms", 3000000_i64) as u64, secret: cm.get::<String>("webhook.secret").ok(), enabled: cm.get_or("webhook.enabled", false) },
    ))
}

