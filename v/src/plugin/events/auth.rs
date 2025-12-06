//! # 认证事件监听器 / Authentication Event Listener
//!
//! 定义认证插件的事件监听器 trait
//! Defines the event listener trait for authentication plugins

use anyhow::Result;
use async_trait::async_trait;

use crate::plugin::pdk::Context;

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
/// use v::plugin::pdk::{Context, AuthEventListener};
/// use async_trait::async_trait;
///
/// pub struct MyAuthListener {
///     // 你的字段
/// }
///
/// #[async_trait]
/// impl AuthEventListener for MyAuthListener {
///     async fn auth_login(&mut self, ctx: &mut Context) -> Result<()> {
///         // 你的实现
///         Ok(())
///     }
///     // ... 实现其他方法
/// }
/// ```
#[async_trait]
pub trait AuthEventListener: Send + Sync {
    /// 用户登录事件 / User login event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户登录信息 / Plugin context containing user login info
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_login(&mut self, ctx: &mut Context) -> Result<()>;

    /// 用户登出事件 / User logout event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID / Plugin context containing user ID
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_logout(&mut self, ctx: &mut Context) -> Result<()>;

    /// 用户被踢出事件 / User kick out event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID和原因 / Plugin context containing user ID and reason
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_kick_out(&mut self, ctx: &mut Context) -> Result<()>;

    /// Token 续期事件 / Token renew event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含Token信息 / Plugin context containing token info
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_renew_timeout(&mut self, ctx: &mut Context) -> Result<()>;

    /// Token 被替换事件 / Token replaced event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含新旧Token信息 / Plugin context containing old and new token info
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_replaced(&mut self, ctx: &mut Context) -> Result<()>;

    /// 用户被封禁事件 / User banned event
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文，包含用户ID和封禁原因 / Plugin context containing user ID and ban reason
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn auth_banned(&mut self, ctx: &mut Context) -> Result<()>;

    /// 自动事件分发 / Auto event dispatch
    ///
    /// 根据事件类型自动调用对应的处理方法
    /// Automatically calls the corresponding handler method based on event type
    ///
    /// # 参数 / Parameters
    /// - `ctx`: 插件上下文 / Plugin context
    ///
    /// # 返回 / Returns
    /// - `Result<()>`: 成功或错误 / Success or error
    async fn dispatch(&mut self, ctx: &mut Context) -> Result<()> {
        let event_type = ctx.event_type();

        match event_type {
            "auth.login" => self.auth_login(ctx).await,
            "auth.logout" => self.auth_logout(ctx).await,
            "auth.kick_out" => self.auth_kick_out(ctx).await,
            "auth.renew_timeout" => self.auth_renew_timeout(ctx).await,
            "auth.replaced" => self.auth_replaced(ctx).await,
            "auth.banned" => self.auth_banned(ctx).await,
            _ => {
                crate::warn!(
                    "⚠️  未知的认证事件类型 / Unknown auth event type: {}",
                    event_type
                );
                ctx.reply(serde_json::json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
                Ok(())
            }
        }
    }
}
