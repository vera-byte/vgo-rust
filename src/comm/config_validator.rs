use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{AppError, AppResult};

/// 配置验证规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigValidationRule {
    pub key: String,
    pub required: bool,
    pub data_type: ConfigDataType,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub allowed_values: Option<Vec<String>>,
    pub regex_pattern: Option<String>,
    pub description: String,
}

/// 配置数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigDataType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
}

/// 应用配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    pub server_host: String,
    pub server_port: u16,
    pub server_workers: Option<usize>,
    pub server_debug: bool,
    pub database_url: Option<String>,
    pub database_max_connections: Option<u32>,
    pub logging_level: String,
    pub logging_json_format: bool,
    pub jwt_secret: Option<String>,
    pub jwt_expiration: Option<u64>,
    pub rate_limit_requests_per_minute: Option<u32>,
    pub cors_allowed_origins: Option<Vec<String>>,
    pub redis_url: Option<String>,
    pub cache_ttl_seconds: Option<u64>,
}

impl Default for AppConfiguration {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            server_workers: Some(4),
            server_debug: false,
            database_url: None,
            database_max_connections: Some(10),
            logging_level: "info".to_string(),
            logging_json_format: false,
            jwt_secret: None,
            jwt_expiration: Some(3600),
            rate_limit_requests_per_minute: Some(60),
            cors_allowed_origins: Some(vec!["*".to_string()]),
            redis_url: None,
            cache_ttl_seconds: Some(300),
        }
    }
}

/// 配置验证器
pub struct ConfigValidator {
    rules: HashMap<String, ConfigValidationRule>,
}

#[allow(dead_code)]
impl ConfigValidator {
    /// 创建新的配置验证器
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        
        // 定义验证规则
        rules.insert("server.host".to_string(), ConfigValidationRule {
            key: "server.host".to_string(),
            required: true,
            data_type: ConfigDataType::String,
            min_value: None,
            max_value: None,
            allowed_values: None,
            regex_pattern: Some(r"^[a-zA-Z0-9.-]+$".to_string()),
            description: "服务器主机地址".to_string(),
        });
        
        rules.insert("server.port".to_string(), ConfigValidationRule {
            key: "server.port".to_string(),
            required: true,
            data_type: ConfigDataType::Integer,
            min_value: Some(1024.0),
            max_value: Some(65535.0),
            allowed_values: None,
            regex_pattern: None,
            description: "服务器端口".to_string(),
        });
        
        rules.insert("logging.level".to_string(), ConfigValidationRule {
            key: "logging.level".to_string(),
            required: true,
            data_type: ConfigDataType::String,
            min_value: None,
            max_value: None,
            allowed_values: Some(vec![
                "trace".to_string(),
                "debug".to_string(),
                "info".to_string(),
                "warn".to_string(),
                "error".to_string(),
            ]),
            regex_pattern: None,
            description: "日志级别".to_string(),
        });
        
        Self { rules }
    }
    
    /// 验证配置值
    pub fn validate_value(&self, key: &str, value: &serde_json::Value) -> AppResult<()> {
        if let Some(rule) = self.rules.get(key) {
            self.validate_against_rule(rule, value)?;
        }
        Ok(())
    }
    
    /// 验证整个配置
    pub fn validate_config(&self, config: &AppConfiguration) -> AppResult<()> {
        // 验证服务器主机
        if config.server_host.is_empty() {
            return Err(AppError::validation("server_host", "服务器主机不能为空"));
        }
        
        // 验证端口范围
        if config.server_port < 1024 {
            return Err(AppError::validation("server_port", "端口必须在1024-65535范围内"));
        }
        
        // 验证工作线程数
        if let Some(workers) = config.server_workers {
            if workers == 0 || workers > 32 {
                return Err(AppError::validation("server_workers", "工作线程数必须在1-32范围内"));
            }
        }
        
        // 验证数据库连接池大小
        if let Some(max_conn) = config.database_max_connections {
            if max_conn == 0 || max_conn > 100 {
                return Err(AppError::validation("database_max_connections", "数据库连接池大小必须在1-100范围内"));
            }
        }
        
        // 验证日志级别
        if config.logging_level.is_empty() {
            return Err(AppError::validation("logging_level", "日志级别不能为空"));
        }
        
        // 验证JWT过期时间
        if let Some(exp) = config.jwt_expiration {
            if exp < 300 || exp > 86400 {
                return Err(AppError::validation("jwt_expiration", "JWT过期时间必须在300-86400秒范围内"));
            }
        }
        
        // 验证请求限流
        if let Some(rate) = config.rate_limit_requests_per_minute {
            if rate == 0 || rate > 1000 {
                return Err(AppError::validation("rate_limit_requests_per_minute", "请求限流必须在1-1000范围内"));
            }
        }
        
        // 验证缓存TTL
        if let Some(ttl) = config.cache_ttl_seconds {
            if ttl == 0 || ttl > 3600 {
                return Err(AppError::validation("cache_ttl_seconds", "缓存TTL必须在1-3600秒范围内"));
            }
        }
        
        Ok(())
    }
    
    /// 根据规则验证值
    fn validate_against_rule(&self, rule: &ConfigValidationRule, value: &serde_json::Value) -> AppResult<()> {
        // 检查数据类型
        match (&rule.data_type, value) {
            (ConfigDataType::String, serde_json::Value::String(s)) => {
                if let Some(pattern) = &rule.regex_pattern {
                    let regex = regex::Regex::new(pattern)
                        .map_err(|e| AppError::validation(&rule.key, format!("正则表达式错误: {}", e)))?;
                    if !regex.is_match(s) {
                        return Err(AppError::validation(&rule.key, "值不匹配正则表达式"));
                    }
                }
                
                if let Some(allowed) = &rule.allowed_values {
                    if !allowed.contains(s) {
                        return Err(AppError::validation(&rule.key, 
                            format!("值必须是以下之一: {}", allowed.join(", "))));
                    }
                }
            }
            (ConfigDataType::Integer, serde_json::Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    let f = i as f64;
                    if let Some(min) = rule.min_value {
                        if f < min {
                            return Err(AppError::validation(&rule.key, 
                                format!("值必须大于等于 {}", min)));
                        }
                    }
                    if let Some(max) = rule.max_value {
                        if f > max {
                            return Err(AppError::validation(&rule.key, 
                                format!("值必须小于等于 {}", max)));
                        }
                    }
                }
            }
            (ConfigDataType::Float, serde_json::Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    if let Some(min) = rule.min_value {
                        if f < min {
                            return Err(AppError::validation(&rule.key, 
                                format!("值必须大于等于 {}", min)));
                        }
                    }
                    if let Some(max) = rule.max_value {
                        if f > max {
                            return Err(AppError::validation(&rule.key, 
                                format!("值必须小于等于 {}", max)));
                        }
                    }
                }
            }
            (ConfigDataType::Boolean, serde_json::Value::Bool(_)) => {
                // 布尔值验证通过
            }
            _ => {
                return Err(AppError::validation(&rule.key, 
                    format!("数据类型不匹配，期望: {:?}", rule.data_type)));
            }
        }
        
        Ok(())
    }
    
    /// 获取所有验证规则
    pub fn get_rules(&self) -> &HashMap<String, ConfigValidationRule> {
        &self.rules
    }
    
    /// 添加自定义验证规则
    pub fn add_rule(&mut self, rule: ConfigValidationRule) {
        self.rules.insert(rule.key.clone(), rule);
    }
}

/// 环境特定配置加载器
#[allow(dead_code)]
pub struct EnvironmentConfigLoader {
    environment: String,
}

#[allow(dead_code)]
impl EnvironmentConfigLoader {
    /// 创建环境配置加载器
    pub fn new() -> Self {
        let environment = std::env::var("VGO_ENV")
            .unwrap_or_else(|_| "development".to_string());
        
        Self { environment }
    }
    
    /// 获取当前环境
    pub fn get_environment(&self) -> &str {
        &self.environment
    }
    
    /// 获取环境特定的配置文件路径
    pub fn get_config_paths(&self) -> Vec<String> {
        vec![
            "config/default.toml".to_string(),
            format!("config/{}.toml", self.environment),
            "config/local.toml".to_string(),
        ]
    }
    
    /// 是否为生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
    
    /// 是否为开发环境
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
    
    /// 是否为测试环境
    pub fn is_test(&self) -> bool {
        self.environment == "test"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_validation() {
        let validator = ConfigValidator::new();
        let config = AppConfiguration::default();
        
        assert!(validator.validate_config(&config).is_ok());
    }
    
    #[test]
    fn test_invalid_port() {
        let mut config = AppConfiguration::default();
        config.server_port = 80; // 小于1024
        
        let validator = ConfigValidator::new();
        assert!(validator.validate_config(&config).is_err());
    }
    
    #[test]
    fn test_environment_loader() {
        let loader = EnvironmentConfigLoader::new();
        let paths = loader.get_config_paths();
        
        assert!(paths.len() >= 3);
        assert!(paths.iter().any(|p| p.contains("default")));
    }
}