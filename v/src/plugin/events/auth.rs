//! # 认证事件监听器 / Authentication Event Listener
//!
//! 定义认证插件的事件监听器 trait
//! Defines the event listener trait for authentication plugins

use anyhow::Result;
use async_trait::async_trait;

use crate::plugin::protocol::{
    BanUserRequest, BanUserResponse, KickOutRequest, KickOutResponse, LoginRequest, LoginResponse,
    LogoutRequest, LogoutResponse, RenewTokenRequest, RenewTokenResponse, TokenReplacedRequest,
    TokenReplacedResponse, ValidateTokenRequest, ValidateTokenResponse,
};

// ============================================================================
// 认证事件监听器 Trait / Authentication Event Listener Trait
// ============================================================================

/// 认证事件监听器 trait / Authentication event listener trait
///
/// 定义所有认证相关事件的处理方法
/// Defines all authentication-related event handler methods
///
/// # 使用示例 / Usage Example
///
/// ```ignore
/// use v::plugin::pdk::AuthEventListener;
/// use v::plugin::protocol::*;
/// use async_trait::async_trait;
///
/// pub struct MyAuthListener;
///
/// #[async_trait]
/// impl AuthEventListener for MyAuthListener {
///     async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse> {
///         // 类型安全的实现
///         Ok(LoginResponse {
///             status: "ok".to_string(),
///             token: "token123".to_string(),
///             uid: req.username.clone(),
///             expires_at: 1234567890,
///         })
///     }
///     // ... 实现其他方法
/// }
/// ```
#[async_trait]
pub trait AuthEventListener: Send + Sync {
    /// 用户登录事件 / User login event
    ///
    /// # 参数 / Parameters
    /// - `req`: 登录请求 / Login request
    ///
    /// # 返回 / Returns
    /// - `Result<LoginResponse>`: 登录响应 / Login response
    async fn auth_login(&mut self, req: &LoginRequest) -> Result<LoginResponse>;

    /// 用户登出事件 / User logout event
    ///
    /// # 参数 / Parameters
    /// - `req`: 登出请求 / Logout request
    ///
    /// # 返回 / Returns
    /// - `Result<LogoutResponse>`: 登出响应 / Logout response
    async fn auth_logout(&mut self, req: &LogoutRequest) -> Result<LogoutResponse>;

    /// 用户被踢出事件 / User kick out event
    ///
    /// # 参数 / Parameters
    /// - `req`: 踢出请求 / Kick out request
    ///
    /// # 返回 / Returns
    /// - `Result<KickOutResponse>`: 踢出响应 / Kick out response
    async fn auth_kick_out(&mut self, req: &KickOutRequest) -> Result<KickOutResponse>;

    /// Token 续期事件 / Token renew event
    ///
    /// # 参数 / Parameters
    /// - `req`: Token 续期请求 / Token renew request
    ///
    /// # 返回 / Returns
    /// - `Result<RenewTokenResponse>`: Token 续期响应 / Token renew response
    async fn auth_renew_token(&mut self, req: &RenewTokenRequest) -> Result<RenewTokenResponse>;

    /// Token 被替换事件 / Token replaced event
    ///
    /// # 参数 / Parameters
    /// - `req`: Token 替换请求 / Token replaced request
    ///
    /// # 返回 / Returns
    /// - `Result<TokenReplacedResponse>`: Token 替换响应 / Token replaced response
    async fn auth_token_replaced(
        &mut self,
        req: &TokenReplacedRequest,
    ) -> Result<TokenReplacedResponse>;

    /// 用户被封禁事件 / User banned event
    ///
    /// # 参数 / Parameters
    /// - `req`: 用户封禁请求 / User ban request
    ///
    /// # 返回 / Returns
    /// - `Result<BanUserResponse>`: 用户封禁响应 / User ban response
    async fn auth_ban_user(&mut self, req: &BanUserRequest) -> Result<BanUserResponse>;

    /// Token 验证事件 / Token validation event
    ///
    /// # 参数 / Parameters
    /// - `req`: Token 验证请求 / Token validation request
    ///
    /// # 返回 / Returns
    /// - `Result<ValidateTokenResponse>`: Token 验证响应 / Token validation response
    async fn auth_validate_token(
        &mut self,
        req: &ValidateTokenRequest,
    ) -> Result<ValidateTokenResponse>;
}
