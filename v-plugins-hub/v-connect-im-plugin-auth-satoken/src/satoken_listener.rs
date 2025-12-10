//! # SaToken 认证监听器实现 / SaToken Auth Listener Implementation
//!
//! 基于 SaToken 的认证事件监听器实现
//! Authentication event listener implementation based on SaToken

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use v::plugin::pdk::AuthEventListener;
use v::plugin::protocol::*;
use v::{debug, info, warn};

// ============================================================================
// 配置结构 / Configuration Structure
// ============================================================================

/// SaToken 认证配置 / SaToken authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaTokenAuthConfig {
    /// SaToken 服务地址 / SaToken service URL
    #[serde(default = "default_satoken_url")]
    pub satoken_url: String,

    /// 请求超时时间（毫秒）/ Request timeout (milliseconds)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Token 有效期（秒）/ Token validity period (seconds)
    #[serde(default = "default_token_ttl")]
    pub token_ttl: i64,
}

fn default_satoken_url() -> String {
    "http://127.0.0.1:8090".to_string()
}

fn default_timeout_ms() -> u64 {
    3000
}

fn default_token_ttl() -> i64 {
    7200 // 2 小时 / 2 hours
}

impl Default for SaTokenAuthConfig {
    fn default() -> Self {
        Self {
            satoken_url: default_satoken_url(),
            timeout_ms: default_timeout_ms(),
            token_ttl: default_token_ttl(),
        }
    }
}

impl SaTokenAuthConfig {
    /// 验证配置有效性 / Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.satoken_url.is_empty() {
            anyhow::bail!("satoken_url 不能为空 / satoken_url cannot be empty");
        }

        if self.timeout_ms == 0 {
            anyhow::bail!("timeout_ms 必须大于 0 / timeout_ms must be greater than 0");
        }

        if self.token_ttl <= 0 {
            anyhow::bail!("token_ttl 必须大于 0 / token_ttl must be greater than 0");
        }

        Ok(())
    }
}

// ============================================================================
// 主结构 / Main Structure
// ============================================================================

/// SaToken 认证事件监听器 / SaToken authentication event listener
pub struct SaTokenAuthListener {
    /// 配置 / Configuration
    pub config: SaTokenAuthConfig,
    /// HTTP 客户端 / HTTP client
    client: reqwest::Client,
}

impl SaTokenAuthListener {
    /// 创建新实例 / Create new instance
    pub fn new(config: SaTokenAuthConfig) -> Result<Self> {
        info!("🔐 初始化 SaToken 认证监听器 / Initializing SaToken auth listener");

        // 验证配置 / Validate configuration
        config.validate()?;

        // 创建 HTTP 客户端 / Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()?;

        info!(
            "✅ SaToken 认证监听器初始化完成 / SaToken auth listener initialized: {}",
            config.satoken_url
        );

        Ok(Self { config, client })
    }

    /// 验证 Token / Validate token
    async fn validate_token(&self, token: &str) -> Result<bool> {
        if token.is_empty() {
            return Ok(false);
        }

        debug!("🔍 验证 Token / Validating token: {}", token);

        let url = format!("{}/v1/sso/auth", self.config.satoken_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("token", token)])
            .send()
            .await?;

        let is_valid = resp.status().is_success();
        debug!("Token 验证结果 / Token validation result: {}", is_valid);

        Ok(is_valid)
    }
}

// ============================================================================
// 实现 AuthEventListener Trait / Implement AuthEventListener Trait
// ============================================================================

#[async_trait]
impl AuthEventListener for SaTokenAuthListener {
    /// 用户登录 / User login
    async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse> {
        info!("🔐 用户登录 / User login: username={}", req.username);

        // 调用 SaToken 登录接口 / Call SaToken login API
        let url = format!("{}/v1/sso/login", self.config.satoken_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "username": req.username,
                "password": req.password,
            }))
            .send()
            .await?;

        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await?;
            let token = data
                .get("data")
                .and_then(|d| d.get("token"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let uid = data
                .get("data")
                .and_then(|d| d.get("uid"))
                .and_then(|u| u.as_str())
                .unwrap_or(&req.username)
                .to_string();

            let expires_at = chrono::Utc::now().timestamp() + self.config.token_ttl;

            info!("✅ 登录成功 / Login successful: uid={}", uid);

            Ok(LoginResponse {
                status: "ok".to_string(),
                token,
                uid,
                expires_at,
            })
        } else {
            warn!("❌ 登录失败 / Login failed: {}", resp.status());
            Ok(LoginResponse {
                status: "error".to_string(),
                token: String::new(),
                uid: String::new(),
                expires_at: 0,
            })
        }
    }

    /// 用户登出 / User logout
    async fn auth_logout(&mut self, req: &LogoutRequest) -> Result<LogoutResponse> {
        info!("👋 用户登出 / User logout: uid={}", req.uid);

        // 调用 SaToken 登出接口 / Call SaToken logout API
        let url = format!("{}/v1/sso/logout", self.config.satoken_url);
        let _resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "token": req.token,
            }))
            .send()
            .await?;

        info!("✅ 登出成功 / Logout successful");

        Ok(LogoutResponse {
            status: "ok".to_string(),
        })
    }

    /// 踢出用户 / Kick out user
    async fn auth_kick_out(&mut self, req: &KickOutRequest) -> Result<KickOutResponse> {
        info!("⚠️  踢出用户 / Kick out user: uid={}", req.uid);

        // 调用 SaToken 踢出接口 / Call SaToken kick out API
        let url = format!("{}/v1/sso/kickout", self.config.satoken_url);
        let _resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "uid": req.uid,
            }))
            .send()
            .await?;

        info!("✅ 踢出成功 / Kick out successful");

        Ok(KickOutResponse {
            status: "ok".to_string(),
        })
    }

    /// Token 续期 / Renew token
    async fn auth_renew_token(&mut self, req: &RenewTokenRequest) -> Result<RenewTokenResponse> {
        info!("🔄 Token 续期 / Renew token: old_token={}", req.old_token);

        // 先验证 token / First validate token
        let is_valid = self.validate_token(&req.old_token).await?;

        if is_valid {
            let new_expires_at = chrono::Utc::now().timestamp() + self.config.token_ttl;

            info!("✅ Token 续期成功 / Token renewed successfully");

            Ok(RenewTokenResponse {
                status: "ok".to_string(),
                new_token: req.old_token.clone(), // 实际应该生成新 token
                expires_at: new_expires_at,
            })
        } else {
            warn!("❌ Token 无效，续期失败 / Invalid token, renew failed");

            Ok(RenewTokenResponse {
                status: "error".to_string(),
                new_token: String::new(),
                expires_at: 0,
            })
        }
    }

    /// Token 被替换 / Token replaced
    async fn auth_token_replaced(
        &mut self,
        req: &TokenReplacedRequest,
    ) -> Result<TokenReplacedResponse> {
        info!(
            "🔄 Token 被替换 / Token replaced: old_token={}, new_token={}",
            req.old_token, req.new_token
        );

        // 记录 token 替换事件 / Log token replacement event
        debug!("Token 替换事件已记录 / Token replacement event logged");

        Ok(TokenReplacedResponse {
            status: "ok".to_string(),
        })
    }

    /// 封禁用户 / Ban user
    async fn auth_ban_user(&mut self, req: &BanUserRequest) -> Result<BanUserResponse> {
        info!(
            "🚫 封禁用户 / Ban user: uid={}, reason={}",
            req.uid, req.reason
        );

        // 调用 SaToken 封禁接口 / Call SaToken ban API
        let url = format!("{}/v1/sso/ban", self.config.satoken_url);
        let _resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "uid": req.uid,
                "reason": req.reason,
                "ban_until": req.ban_until,
            }))
            .send()
            .await?;

        info!("✅ 封禁成功 / Ban successful");

        Ok(BanUserResponse {
            status: "ok".to_string(),
        })
    }

    /// Token 验证 / Token validation
    async fn auth_validate_token(
        &mut self,
        req: &ValidateTokenRequest,
    ) -> Result<ValidateTokenResponse> {
        info!("🔍 验证 Token / Validate token: {}", req.token);

        // 调用 SaToken 验证接口 / Call SaToken validation API
        let url = format!("{}/v1/sso/checkToken", self.config.satoken_url);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "token": req.token,
            }))
            .timeout(std::time::Duration::from_millis(self.config.timeout_ms))
            .send()
            .await;

        match resp {
            Ok(response) => {
                if response.status().is_success() {
                    // 解析响应 / Parse response
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        let is_valid = data
                            .get("data")
                            .and_then(|d| d.get("isValid"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if is_valid {
                            let uid = data
                                .get("data")
                                .and_then(|d| d.get("uid"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let expires_at = data
                                .get("data")
                                .and_then(|d| d.get("expiresAt"))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);

                            info!("✅ Token 有效 / Token valid: uid={}", uid);

                            return Ok(ValidateTokenResponse {
                                status: "ok".to_string(),
                                valid: true,
                                uid,
                                expires_at,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "⚠️  SaToken 验证请求失败 / SaToken validation request failed: {}",
                    e
                );
            }
        }

        info!("❌ Token 无效 / Token invalid");

        Ok(ValidateTokenResponse {
            status: "ok".to_string(),
            valid: false,
            uid: String::new(),
            expires_at: 0,
        })
    }
}
