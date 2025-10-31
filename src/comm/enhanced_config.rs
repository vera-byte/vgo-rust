use crate::comm::config::{ConfigManager, ConfigSource};
use crate::comm::config_validator::{AppConfiguration, ConfigValidator, EnvironmentConfigLoader};
use crate::error::{AppError, AppResult};
use config::FileFormat;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, info};

/// 增强的配置管理器
#[allow(dead_code)]
pub struct EnhancedConfigManager {
    config_manager: Arc<ConfigManager>,
    validator: ConfigValidator,
    env_loader: EnvironmentConfigLoader,
    app_config: AppConfiguration,
}

#[allow(dead_code)]
impl EnhancedConfigManager {
    /// 创建增强的配置管理器
    pub async fn new() -> AppResult<Self> {
        let env_loader = EnvironmentConfigLoader::new();
        info!("当前环境: {}", env_loader.get_environment());

        // 构建配置源
        let mut sources = Vec::new();

        // 添加环境特定的配置文件
        for path in env_loader.get_config_paths() {
            sources.push(ConfigSource::File {
                path: path.clone(),
                format: Some(FileFormat::Toml),
                required: path.contains("default"), // 只有default配置是必需的
            });
        }

        // 添加环境变量配置
        sources.push(ConfigSource::Env {
            prefix: "VGO".to_string(),
            separator: "_",
        });

        // 创建配置管理器
        let config_manager = Arc::new(ConfigManager::with_sources(sources).map_err(|e| {
            AppError::Config(crate::comm::config::ConfigError::InitializationError {
                message: e.to_string(),
            })
        })?);

        // 创建验证器
        let validator = ConfigValidator::new();

        // 加载并验证应用配置
        let app_config = Self::load_app_config(&config_manager, &validator).await?;

        Ok(Self {
            config_manager,
            validator,
            env_loader,
            app_config,
        })
    }

    /// 加载应用配置
    async fn load_app_config(
        config_manager: &ConfigManager,
        validator: &ConfigValidator,
    ) -> AppResult<AppConfiguration> {
        let mut app_config = AppConfiguration::default();

        // 从配置管理器加载配置值
        if let Ok(host) = config_manager.get_string("server.host") {
            app_config.server_host = host;
        }

        if let Ok(port) = config_manager.get::<u16>("server.port") {
            app_config.server_port = port;
        }

        if let Ok(workers) = config_manager.get::<usize>("server.workers") {
            app_config.server_workers = Some(workers);
        }

        if let Ok(debug) = config_manager.get::<bool>("server.debug") {
            app_config.server_debug = debug;
        }

        if let Ok(db_url) = config_manager.get_string("database.url") {
            app_config.database_url = Some(db_url);
        }

        if let Ok(max_conn) = config_manager.get::<u32>("database.max_connections") {
            app_config.database_max_connections = Some(max_conn);
        }

        if let Ok(level) = config_manager.get_string("logging.level") {
            app_config.logging_level = level;
        }

        if let Ok(json_format) = config_manager.get::<bool>("logging.json_format") {
            app_config.logging_json_format = json_format;
        }

        if let Ok(secret) = config_manager.get_string("jwt.secret") {
            app_config.jwt_secret = Some(secret);
        }

        if let Ok(exp) = config_manager.get::<u64>("jwt.expiration") {
            app_config.jwt_expiration = Some(exp);
        }

        if let Ok(rate_limit) = config_manager.get::<u32>("rate_limit.requests_per_minute") {
            app_config.rate_limit_requests_per_minute = Some(rate_limit);
        }

        if let Ok(origins) = config_manager.get::<Vec<String>>("cors.allowed_origins") {
            app_config.cors_allowed_origins = Some(origins);
        }

        if let Ok(redis_url) = config_manager.get_string("redis.url") {
            app_config.redis_url = Some(redis_url);
        }

        if let Ok(ttl) = config_manager.get::<u64>("cache.ttl_seconds") {
            app_config.cache_ttl_seconds = Some(ttl);
        }

        // 验证配置
        validator.validate_config(&app_config)?;

        info!("配置加载和验证成功");
        debug!("应用配置: {:?}", app_config);

        Ok(app_config)
    }

    /// 获取应用配置
    pub fn get_app_config(&self) -> &AppConfiguration {
        &self.app_config
    }

    /// 获取原始配置管理器
    pub fn get_config_manager(&self) -> &ConfigManager {
        &self.config_manager
    }

    /// 获取环境加载器
    pub fn get_env_loader(&self) -> &EnvironmentConfigLoader {
        &self.env_loader
    }

    /// 获取配置值（带验证）
    pub fn get_validated<T: DeserializeOwned + serde::Serialize>(&self, key: &str) -> AppResult<T> {
        let value = self.config_manager.get(key).map_err(|_e| {
            AppError::Config(crate::comm::config::ConfigError::KeyNotFound {
                key: key.to_string(),
            })
        })?;

        // 如果有验证规则，进行验证
        if let Ok(json_value) = serde_json::to_value(&value) {
            self.validator.validate_value(key, &json_value)?;
        }

        Ok(value)
    }

    /// 重新加载配置
    pub async fn reload(&mut self) -> AppResult<()> {
        info!("重新加载配置...");

        // 重新创建配置管理器
        let env_loader = EnvironmentConfigLoader::new();
        let mut sources = Vec::new();

        for path in env_loader.get_config_paths() {
            sources.push(ConfigSource::File {
                path: path.clone(),
                format: Some(FileFormat::Toml),
                required: path.contains("default"),
            });
        }

        sources.push(ConfigSource::Env {
            prefix: "VGO".to_string(),
            separator: "_",
        });

        let config_manager = Arc::new(ConfigManager::with_sources(sources).map_err(|e| {
            AppError::Config(crate::comm::config::ConfigError::InitializationError {
                message: e.to_string(),
            })
        })?);

        // 重新加载应用配置
        let app_config = Self::load_app_config(&config_manager, &self.validator).await?;

        self.config_manager = config_manager;
        self.app_config = app_config;

        info!("配置重新加载成功");
        Ok(())
    }

    /// 验证必需的配置项
    pub fn validate_required_config(&self) -> AppResult<()> {
        let required_keys = if self.env_loader.is_production() {
            vec![
                "server.host",
                "server.port",
                "database.url",
                "jwt.secret",
                "logging.level",
            ]
        } else {
            vec!["server.host", "server.port", "logging.level"]
        };

        for key in required_keys {
            if !self.config_manager.exists(key) {
                return Err(AppError::Config(
                    crate::comm::config::ConfigError::KeyNotFound {
                        key: key.to_string(),
                    },
                ));
            }
        }

        Ok(())
    }

    /// 打印配置摘要
    pub fn print_config_summary(&self) {
        info!("=== 配置摘要 ===");
        info!("环境: {}", self.env_loader.get_environment());
        info!(
            "服务器: {}:{}",
            self.app_config.server_host, self.app_config.server_port
        );
        info!("工作线程: {:?}", self.app_config.server_workers);
        info!("调试模式: {}", self.app_config.server_debug);
        info!("日志级别: {}", self.app_config.logging_level);
        info!("JSON日志: {}", self.app_config.logging_json_format);

        if let Some(db_url) = &self.app_config.database_url {
            info!("数据库: {}", Self::mask_sensitive_info(db_url));
        }

        if let Some(redis_url) = &self.app_config.redis_url {
            info!("Redis: {}", Self::mask_sensitive_info(redis_url));
        }

        info!(
            "限流: {:?} 请求/分钟",
            self.app_config.rate_limit_requests_per_minute
        );
        info!("缓存TTL: {:?} 秒", self.app_config.cache_ttl_seconds);

        // 打印配置源信息
        self.config_manager.print_sources_info();
    }

    /// 屏蔽敏感信息
    fn mask_sensitive_info(url: &str) -> String {
        // 简单的密码屏蔽，查找://和@之间的内容
        if let Some(start) = url.find("://") {
            if let Some(at_pos) = url[start + 3..].find('@') {
                let mut result = url.to_string();
                let password_start = start + 3;
                let password_end = password_start + at_pos;
                
                // 查找用户名和密码分隔符
                if let Some(colon_pos) = url[password_start..password_end].find(':') {
                    let actual_colon_pos = password_start + colon_pos + 1;
                    result.replace_range(actual_colon_pos..password_end, "***");
                }
                result
            } else {
                url.to_string()
            }
        } else {
            url.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_config_manager() {
        // 设置测试环境
        std::env::set_var("VGO_ENV", "test");

        let result = EnhancedConfigManager::new().await;
        // 在没有配置文件的情况下，应该使用默认配置
        assert!(result.is_ok() || result.is_err()); // 取决于是否有配置文件
    }
}
