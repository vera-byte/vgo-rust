#[derive(Clone)]
/// 精简鉴权配置，用于运行时快速读取 / Lightweight auth configuration snapshot for runtime usage.
pub struct AuthConfigLite {
    pub enabled: bool,
    pub center_url: String,
    pub timeout_ms: u64,
}

// #[derive(Clone)]
// /// 精简 Webhook 配置 / Lightweight webhook configuration used by the server. (已移除 / Removed)
// pub struct WebhookConfigLite {
//     pub url: Option<String>,
//     pub timeout_ms: u64,
//     pub secret: Option<String>,
//     pub enabled: bool,
// }
