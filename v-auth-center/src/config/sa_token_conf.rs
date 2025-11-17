//! Sa-Token 配置初始化
//! Sa-Token Configuration Initialization
use crate::event::my_listener::MyListener;
use anyhow::Result;
use sa_token_plugin_actix_web::{
    LoggingListener, MemoryStorage, RedisStorage, SaTokenConfig, SaTokenManager, TokenStyle,
};
use serde::Deserialize;
use std::sync::Arc;
use v::get_global_config_manager;

/// Redis配置
/// Redis Configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub prefix: Option<String>,
}

/// 初始化 Sa-Token
/// Initialize Sa-Token
///
/// 如果提供了 Redis 配置，则使用 Redis 存储
/// If Redis configuration is provided, use Redis storage
/// 否则使用内存存储
/// Otherwise use memory storage
pub async fn init_sa_token(redis_config: Option<&RedisConfig>) -> Result<Arc<SaTokenManager>> {
    // 读取配置 / Read configuration via ConfigManager
    let mgr = get_global_config_manager()?;
    let token_name: String = mgr.get_or("sa_token.token_name", "Authorization".to_string());
    let timeout_seconds: i64 = mgr.get_or("sa_token.timeout_seconds", 86400_i64);
    let token_style_str: String = mgr.get_or("sa_token.token_style", "random64".to_string());
    let auto_renew: bool = mgr.get_or("sa_token.auto_renew", true);
    let storage_type: String = mgr.get_or("sa_token.storage", "memory".to_string());

    // TokenStyle 映射 / TokenStyle mapping
    let token_style = match token_style_str.to_lowercase().as_str() {
        "random64" => TokenStyle::Random64,
        "jwt" => TokenStyle::Jwt,
        "uuid" => TokenStyle::Uuid,
        "hash" => TokenStyle::Hash,
        "random128" => TokenStyle::Random128,
        "random32" => TokenStyle::Random32,
        "simple_uuid" => TokenStyle::SimpleUuid,
        "tik" => TokenStyle::Tik,
        _ => TokenStyle::Timestamp,
    };

    // 创建配置构建器 / Create config builder
    let mut config_builder = SaTokenConfig::builder()
        .register_listener(Arc::new(MyListener))
        .register_listener(Arc::new(LoggingListener))
        .token_name(token_name)
        .timeout(timeout_seconds)
        .token_style(token_style)
        .auto_renew(auto_renew);

    // 根据配置选择存储方式 / Choose storage method based on configuration
    if let Some(_redis_cfg) = redis_config {
        match RedisStorage::new(
            &_redis_cfg.url,
            _redis_cfg
                .prefix
                .clone()
                .unwrap_or_else(|| "sa_token:".to_string()),
        )
        .await
        {
            Ok(redis_storage) => {
                config_builder = config_builder.storage(Arc::new(redis_storage));
                tracing::info!("Using Redis storage: {}", _redis_cfg.url);
            }
            Err(e) => {
                tracing::warn!("Redis init failed, falling back to memory storage: {}", e);
                config_builder = config_builder.storage(Arc::new(MemoryStorage::new()));
            }
        }
    } else if storage_type.eq_ignore_ascii_case("redis") {
        let url: String = mgr.get_or("sa_token.redis.url", "redis://127.0.0.1/".to_string());
        let prefix: String = mgr.get_or("sa_token.redis.prefix", "sa_token:".to_string());
        match RedisStorage::new(&url, prefix).await {
            Ok(redis_storage) => {
                config_builder = config_builder.storage(Arc::new(redis_storage));
                tracing::info!("Using Redis storage: {}", url);
            }
            Err(e) => {
                tracing::warn!("Redis init failed, falling back to memory storage: {}", e);
                config_builder = config_builder.storage(Arc::new(MemoryStorage::new()));
            }
        }
    } else {
        // 使用内存存储 / Use memory storage
        config_builder = config_builder.storage(Arc::new(MemoryStorage::new()));
        tracing::info!("使用内存存储");
        tracing::info!("Using memory storage");
    }

    // 构建 SaTokenManager
    // Build SaTokenManager
    let manager = config_builder.build();

    Ok(Arc::new(manager))
}
