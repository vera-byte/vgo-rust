//! # SaToken 认证插件 / SaToken Authentication Plugin
//!
//! 基于 SaToken 的认证插件，提供用户登录、登出、Token 验证等功能
//! Authentication plugin based on SaToken, providing login, logout, token validation, etc.

mod satoken_listener;

use anyhow::Result;
use v::info;
use v::plugin::pdk::run_auth_server;

use satoken_listener::{SaTokenAuthConfig, SaTokenAuthListener};

#[tokio::main]
async fn main() -> Result<()> {
    info!("🔐 启动 SaToken 认证插件 / Starting SaToken Auth Plugin");

    run_auth_server::<SaTokenAuthListener, SaTokenAuthConfig, _>(|config| {
        info!("📝 使用配置 / Using config: {:?}", config);

        // 验证配置 / Validate configuration
        config.validate()?;

        // 创建监听器 / Create listener
        SaTokenAuthListener::new(config)
    })
    .await
}
