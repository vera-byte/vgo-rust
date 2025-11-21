/*
模块说明(Module Description):
- 中文: 读取配置，映射令牌样式与参数，依据显式传入的 Redis 配置或 `sa_token.storage` 决定使用 RedisStorage 或 MemoryStorage。
- English: Reads configuration, maps token style and parameters, and chooses RedisStorage or MemoryStorage based on explicit Redis config or `sa_token.storage`.
*/
//! Sa-Token 配置初始化
//! Sa-Token Configuration Initialization
use crate::event::my_listener::MyListener;
type SaResult<T> = std::result::Result<T, v::comm::config::ConfigError>;
use sa_token_plugin_actix_web::{
    LoggingListener, MemoryStorage, OAuth2Manager, RedisStorage, SaStorage, SaTokenConfig,
    SaTokenManager, TokenStyle,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use v::get_global_config_manager;

/// Redis配置 / Redis Configuration
/// 输入参数(Input): 来自配置或启动入口的 Redis 连接 `url` 与可选键前缀 `prefix`
/// 返回值(Return): 被 `init_sa_token` 作为优先的显式存储配置使用
/// 注意(Notes): 未提供 `prefix` 时默认使用 `"sa_token:"`
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub prefix: Option<String>,
}

/// 初始化 Sa-Token / Initialize Sa-Token
/// 返回(Return): `Result<Arc<SaTokenManager>>` 初始化后的 Sa-Token 管理器
/// 异常(Errors): 配置读取失败; Redis 初始化失败时回退到内存并记录日志
/// 复杂度(Complexity): O(1) —— 单次配置读取与单次后端初始化
/// 示例(Example):
/// - 中文: `let mgr = init_sa_token(Some(&RedisConfig{ url: "redis://127.0.0.1/0".into(), prefix: None })).await?;`
/// - English: `let mgr = init_sa_token(Some(&RedisConfig{ url: "redis://127.0.0.1/0".into(), prefix: None })).await?;`
pub async fn init_sa_token() -> SaResult<Arc<SaTokenManager>> {
    // 步骤1: 读取 Sa-Token 相关配置 (名称/超时/样式/续期/存储类型)
    let cfg = read_sa_cfg()?;
    // 步骤2: 选择存储后端 (优先使用显式 Redis 配置, 其次按 sa_token.storage)
    let storage = build_storage(&cfg.storage_type).await?;
    // 步骤3: 构建 Sa-Token 管理器并注册监听器
    let manager = SaTokenConfig::builder()
        .register_listener(Arc::new(MyListener))
        .register_listener(Arc::new(LoggingListener))
        .token_name(cfg.token_name)
        .timeout(cfg.timeout_seconds)
        .token_style(cfg.token_style)
        .auto_renew(cfg.auto_renew)
        .storage(storage)
        .build();
    Ok(Arc::new(manager))
}

/// 通用配置结构 / Common configuration structure
/// 字段说明(Fields):
/// - token_name: 令牌名称 / Token name
/// - timeout_seconds: 过期时间(秒) / Expiration time in seconds
/// - token_style: 令牌样式枚举 / Token style enum
/// - auto_renew: 是否自动续期 / Auto renew flag
/// - storage_type: 存储类型("redis"/"memory") / Storage type
struct SaCfg {
    token_name: String,
    timeout_seconds: i64,
    token_style: TokenStyle,
    auto_renew: bool,
    storage_type: String,
}

/// 读取 Sa-Token 配置 / Read Sa-Token configuration
/// 输入(Input): 无 (使用全局配置管理器)
/// 返回(Return): `SaCfg` 聚合配置
/// 异常(Errors): 配置管理器不可用时返回错误
/// 复杂度(Complexity): O(1)
fn read_sa_cfg() -> SaResult<SaCfg> {
    let mgr = get_global_config_manager()?;
    let token_name: String = mgr.get_or("sa_token.token_name", "Authorization".to_string());
    let timeout_seconds: i64 = mgr.get_or("sa_token.timeout_seconds", 86400_i64);
    let token_style_str: String = mgr.get_or("sa_token.token_style", "random64".to_string());
    let auto_renew: bool = mgr.get_or("sa_token.auto_renew", true);
    let storage_type: String = mgr.get_or("sa_token.storage", "memory".to_string());
    Ok(SaCfg {
        token_name,
        timeout_seconds,
        token_style: map_token_style(&token_style_str),
        auto_renew,
        storage_type,
    })
}

/// 样式映射 / Map token style
/// 输入(Input): 样式字符串 (random64/jwt/uuid/hash/random128/random32/simple_uuid/tik/timestamp)
/// 返回(Return): 对应的 `TokenStyle`
/// 复杂度(Complexity): O(1)
fn map_token_style(s: &str) -> TokenStyle {
    match s.to_lowercase().as_str() {
        "random64" => TokenStyle::Random64,
        "jwt" => TokenStyle::Jwt,
        "uuid" => TokenStyle::Uuid,
        "hash" => TokenStyle::Hash,
        "random128" => TokenStyle::Random128,
        "random32" => TokenStyle::Random32,
        "simple_uuid" => TokenStyle::SimpleUuid,
        "tik" => TokenStyle::Tik,
        _ => TokenStyle::Timestamp,
    }
}

/// 构建存储后端 / Build storage backend
/// 输入(Input):
/// - `redis_config`: 可选显式 Redis 配置 (优先级最高)
/// - `storage_type`: 配置中的存储类型字符串("redis" 或其他)
/// 返回(Return): `Arc<dyn SaStorage>` 已初始化的存储实例
/// 异常(Errors): Redis 初始化失败将回退内存并记录日志；配置读取失败返回错误
/// 示例(Example): `let storage = build_storage(Some(&rc), "redis").await?;`
async fn build_storage(storage_type: &str) -> SaResult<Arc<dyn SaStorage>> {
    let redis_config = v::get_config_safe::<RedisConfig>("redis").ok();
    if let Some(rc) = redis_config {
        let prefix = rc.prefix.clone().unwrap_or_else(|| "sa_token:".to_string());
        match timeout(Duration::from_secs(1), RedisStorage::new(&rc.url, prefix)).await {
            Ok(Ok(rs)) => {
                tracing::info!("Using Redis storage: {}", rc.url);
                Ok(Arc::new(rs))
            }
            Ok(Err(e)) => {
                tracing::warn!("Redis init failed, falling back to memory storage: {}", e);
                Ok(Arc::new(MemoryStorage::new()))
            }
            Err(_) => {
                tracing::warn!(
                    "Redis init timeout, falling back to memory storage: {}",
                    rc.url
                );
                Ok(Arc::new(MemoryStorage::new()))
            }
        }
    // 情况2: 配置声明 storage=redis，读取 sa_token.redis.* 并尝试初始化
    } else if storage_type.eq_ignore_ascii_case("redis") {
        let mgr = get_global_config_manager()?;
        let url: String = mgr.get_or("sa_token.redis.url", "redis://127.0.0.1/".to_string());
        let prefix: String = mgr.get_or("sa_token.redis.prefix", "sa_token:".to_string());
        match timeout(Duration::from_secs(1), RedisStorage::new(&url, prefix)).await {
            Ok(Ok(rs)) => {
                tracing::info!("Using Redis storage: {}", url);
                Ok(Arc::new(rs))
            }
            Ok(Err(e)) => {
                tracing::warn!("Redis init failed, falling back to memory storage: {}", e);
                Ok(Arc::new(MemoryStorage::new()))
            }
            Err(_) => {
                tracing::warn!(
                    "Redis init timeout, falling back to memory storage: {}",
                    url
                );
                Ok(Arc::new(MemoryStorage::new()))
            }
        }
    // 情况3: 默认使用内存存储
    } else {
        tracing::info!("Using memory storage");
        Ok(Arc::new(MemoryStorage::new()))
    }
}
// 初始化 Sa-Token OAuth2 管理器 / Initialize Sa-Token OAuth2 manager
/// 返回(Return): `OAuth2Manager` 已配置的 OAuth2 管理器实例
/// 异常(Errors): 存储初始化失败将返回错误
/// 示例(Example): `let oauth2 = init_sa_token_oath2().await?;`
pub async fn init_sa_token_oath2() -> SaResult<OAuth2Manager> {
    // 步骤1: 读取 Sa-Token 相关配置 (名称/超时/样式/续期/存储类型)
    let cfg = read_sa_cfg()?;
    // 步骤2: 选择存储后端 (优先使用显式 Redis 配置, 其次按 sa_token.storage)
    let storage = build_storage(&cfg.storage_type).await?;
    // 读取 OAuth2 相关 TTL 配置 / Read OAuth2 TTL config
    let mgr = get_global_config_manager()?;
    let ttl_access: i64 = mgr.get_or("sa_token.oauth2.ttl_access", 600_i64);
    let ttl_refresh: i64 = mgr.get_or("sa_token.oauth2.ttl_refresh", 3600_i64);
    let ttl_remember: i64 = mgr.get_or("sa_token.oauth2.ttl_remember", 2592000_i64);
    let oauth2 = OAuth2Manager::new(storage).with_ttl(ttl_access, ttl_refresh, ttl_remember);
    Ok(oauth2)
}
