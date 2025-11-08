use anyhow::{anyhow, Result};
use config::{Config, ConfigBuilder, Environment, File, FileFormat};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

lazy_static! {
    static ref CONFIG_CACHE: RwLock<HashMap<String, Arc<Config>>> = RwLock::new(HashMap::new());
    static ref GLOBAL_CONFIG_MANAGER: RwLock<Option<Arc<ConfigManager>>> = RwLock::new(None);
}

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ConfigError {
    #[error("配置文件不存在: {path}")]
    FileNotFound { path: String },
    #[error("配置格式错误: {message}")]
    FormatError { message: String },
    #[error("配置项 '{key}' 不存在")]
    KeyNotFound { key: String },
    #[error("配置项 '{key}' 类型转换失败: {message}")]
    TypeConversionError { key: String, message: String },
    #[error("环境变量配置错误: {message}")]
    EnvironmentError { message: String },
    #[error("配置初始化失败: {message}")]
    InitializationError { message: String },
}

/// 配置数据源信息
#[derive(Debug, Clone)]
pub struct ConfigSourceInfo {
    pub source_type: String,
    pub description: String,
    pub priority: u8,
    pub loaded: bool,
}

/// 配置管理器
pub struct ConfigManager {
    config: Config,
    sources_info: Vec<ConfigSourceInfo>,
}

impl ConfigManager {
    /// 创建配置管理器
    pub fn new() -> Result<Self> {
        Self::with_sources(vec![])
    }

    /// 使用指定的配置源创建配置管理器
    pub fn with_sources(sources: Vec<ConfigSource>) -> Result<Self> {
        let mut builder = Config::builder();
        let mut sources_info = Vec::new();

        // 添加默认配置源（按优先级从低到高，后添加者优先生效）
        // 目标优先级：1. 环境变量 > 2. production.toml > 3. default.toml > 4. development.toml
        // 因此添加顺序应为：development.toml -> default.toml -> production.toml -> 环境变量
        let default_sources = vec![
            ConfigSource::File {
                path: "config/development.toml".to_string(),
                format: Some(FileFormat::Toml),
                required: false,
            },
            ConfigSource::File {
                path: "config/default.toml".to_string(),
                format: Some(FileFormat::Toml),
                required: false,
            },
            ConfigSource::File {
                path: "config/production.toml".to_string(),
                format: Some(FileFormat::Toml),
                required: false,
            },
            ConfigSource::Env {
                prefix: "V".to_string(),
                separator: "_",
            },
        ];

        let mut priority = 1u8;

        // 预处理配置源，检查文件是否存在
        let mut valid_sources: Vec<(ConfigSource, ConfigSourceInfo)> = Vec::new();
        for source in default_sources.into_iter().chain(sources) {
            let source_info = source.get_source_info(priority);

            // 对于文件源，检查文件是否存在
            let should_add = match &source {
                ConfigSource::File { path, required, .. } => {
                    let file_exists = std::path::Path::new(path).exists();
                    if !file_exists && !required {
                        // 可选文件不存在，记录但不添加
                        sources_info.push(ConfigSourceInfo {
                            loaded: false,
                            source_type: source_info.source_type.clone(),
                            description: source_info.description.clone(),
                            priority: source_info.priority,
                        });
                        false
                    } else if !file_exists && *required {
                        // 必需文件不存在，返回错误
                        return Err(anyhow!("必需的配置文件不存在: {}", path));
                    } else {
                        true
                    }
                }
                _ => true, // 非文件源直接添加
            };

            if should_add {
                valid_sources.push((source, source_info));
            }
            priority += 1;
        }

        // 添加有效的配置源
        for (source, source_info) in valid_sources {
            match source.add_to_builder(builder) {
                Ok(new_builder) => {
                    builder = new_builder;
                    sources_info.push(ConfigSourceInfo {
                        loaded: true,
                        ..source_info
                    });
                }
                Err(e) => {
                    return Err(anyhow!("添加配置源失败: {}", e));
                }
            }
        }

        let config = builder
            .build()
            .map_err(|e| anyhow!("构建配置失败: {}", e))?;
        Ok(Self {
            config,
            sources_info,
        })
    }

    /// 获取指定 key 的配置值
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        self.config
            .get(key)
            .map_err(|e| anyhow!("获取配置 '{}' 失败: {}", key, e))
    }

    /// 获取指定 key 的配置值，如果不存在返回默认值
    #[allow(dead_code)]
    pub fn get_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// 安全获取配置值，返回详细错误信息
    pub fn get_safe<T: DeserializeOwned>(&self, key: &str) -> std::result::Result<T, ConfigError> {
        self.config.get(key).map_err(|e| {
            if e.to_string().contains("not found") {
                ConfigError::KeyNotFound {
                    key: key.to_string(),
                }
            } else {
                ConfigError::TypeConversionError {
                    key: key.to_string(),
                    message: e.to_string(),
                }
            }
        })
    }

    /// 获取字符串配置值
    pub fn get_string(&self, key: &str) -> Result<String> {
        self.get(key)
    }
    /// 获取整数配置值
    #[allow(dead_code)]
    pub fn get_int(&self, key: &str) -> Result<i64> {
        self.get(key)
    }
    /// 获取浮点数配置值
    #[allow(dead_code)]
    pub fn get_float(&self, key: &str) -> Result<f64> {
        self.get(key)
    }
    /// 获取布尔配置值
    #[allow(dead_code)]
    pub fn get_bool(&self, key: &str) -> Result<bool> {
        self.get(key)
    }

    /// 检查配置项是否存在
    #[allow(dead_code)]
    pub fn exists(&self, key: &str) -> bool {
        self.config.get::<serde_json::Value>(key).is_ok()
    }

    /// 获取所有配置的克隆（用于调试）
    #[allow(dead_code)]
    pub fn get_all(&self) -> HashMap<String, serde_json::Value> {
        self.config
            .clone()
            .try_deserialize::<HashMap<String, serde_json::Value>>()
            .unwrap_or_default()
    }

    /// 获取所有配置源信息
    #[allow(dead_code)]
    pub fn get_sources_info(&self) -> &Vec<ConfigSourceInfo> {
        &self.sources_info
    }
    /// 获取当前活跃的配置源（已成功加载的）
    #[allow(dead_code)]
    pub fn get_active_sources(&self) -> Vec<&ConfigSourceInfo> {
        self.sources_info
            .iter()
            .filter(|info| info.loaded)
            .collect()
    }
    /// 获取失败的配置源
    #[allow(dead_code)]
    pub fn get_failed_sources(&self) -> Vec<&ConfigSourceInfo> {
        self.sources_info
            .iter()
            .filter(|info| !info.loaded)
            .collect()
    }
    /// 获取配置源统计信息
    #[allow(dead_code)]
    pub fn get_sources_stats(&self) -> (usize, usize, usize) {
        let total = self.sources_info.len();
        let loaded = self.sources_info.iter().filter(|info| info.loaded).count();
        let failed = total - loaded;
        (total, loaded, failed)
    }
    /// 打印配置源详细信息
    #[allow(dead_code)]
    pub fn print_sources_info(&self) {
        println!("配置源信息:");
        println!("============");
        for (index, info) in self.sources_info.iter().enumerate() {
            let status = if info.loaded {
                "✓ 已加载"
            } else {
                "✗ 失败"
            };
            println!(
                "{}. {} - {} (优先级: {})",
                index + 1,
                info.source_type,
                status,
                info.priority
            );
            println!("   描述: {}", info.description);
            println!();
        }

        let (total, loaded, failed) = self.get_sources_stats();
        println!(
            "统计: 总计 {} 个配置源，成功 {} 个，失败 {} 个",
            total, loaded, failed
        );
    }

    /// 验证必需的配置项
    #[allow(dead_code)]
    pub fn validate_required_keys(
        &self,
        required_keys: &[&str],
    ) -> std::result::Result<(), ConfigError> {
        for key in required_keys {
            if !self.exists(key) {
                return Err(ConfigError::KeyNotFound {
                    key: key.to_string(),
                });
            }
        }
        Ok(())
    }
}

/// 配置源类型
#[allow(dead_code)]
pub enum ConfigSource {
    /// 文件配置源
    File {
        path: String,
        format: Option<FileFormat>,
        required: bool,
    },
    /// 环境变量配置源
    Env {
        prefix: String,
        separator: &'static str,
    },
    /// 内存配置源（HashMap）
    Memory(HashMap<String, serde_json::Value>),
    /// 字符串配置源
    String { content: String, format: FileFormat },
}

impl ConfigSource {
    /// 获取配置源信息
    pub fn get_source_info(&self, priority: u8) -> ConfigSourceInfo {
        match self {
            ConfigSource::File {
                path,
                format,
                required,
            } => {
                let format_str = match format {
                    Some(FileFormat::Toml) => "TOML",
                    Some(FileFormat::Yaml) => "YAML",
                    Some(FileFormat::Json) => "JSON",
                    Some(FileFormat::Ini) => "INI",
                    Some(FileFormat::Ron) => "RON",
                    Some(FileFormat::Json5) => "JSON5",
                    None => "Auto-detect",
                    _ => "Unknown",
                };
                ConfigSourceInfo {
                    source_type: "File".to_string(),
                    description: format!(
                        "文件配置源: {} (格式: {}, 必需: {})",
                        path, format_str, required
                    ),
                    priority,
                    loaded: false,
                }
            }
            ConfigSource::Env { prefix, separator } => ConfigSourceInfo {
                source_type: "Environment".to_string(),
                description: format!("环境变量配置源: 前缀={}, 分隔符={}", prefix, separator),
                priority,
                loaded: false,
            },
            ConfigSource::Memory(map) => ConfigSourceInfo {
                source_type: "Memory".to_string(),
                description: format!("内存配置源: {} 个配置项", map.len()),
                priority,
                loaded: false,
            },
            ConfigSource::String { format, .. } => {
                let format_str = match format {
                    FileFormat::Toml => "TOML",
                    FileFormat::Yaml => "YAML",
                    FileFormat::Json => "JSON",
                    FileFormat::Ini => "INI",
                    FileFormat::Ron => "RON",
                    FileFormat::Json5 => "JSON5",
                    _ => "Unknown",
                };
                ConfigSourceInfo {
                    source_type: "String".to_string(),
                    description: format!("字符串配置源: 格式={}", format_str),
                    priority,
                    loaded: false,
                }
            }
        }
    }

    pub fn add_to_builder(
        self,
        builder: ConfigBuilder<config::builder::DefaultState>,
    ) -> Result<ConfigBuilder<config::builder::DefaultState>> {
        match self {
            ConfigSource::File {
                path,
                format,
                required,
            } => {
                let file_source = if let Some(format) = format {
                    File::with_name(&path).format(format)
                } else {
                    File::with_name(&path)
                };
                if required {
                    Ok(builder.add_source(file_source.required(true)))
                } else {
                    Ok(builder.add_source(file_source))
                }
            }
            ConfigSource::Env { prefix, separator } => Ok(builder.add_source(
                Environment::with_prefix(&prefix)
                    .separator(separator)
                    .prefix_separator("_")
                    .ignore_empty(true),
            )),
            ConfigSource::Memory(map) => {
                let json_content = serde_json::to_string(&map)
                    .map_err(|e| anyhow!("序列化内存配置失败: {}", e))?;
                Ok(builder.add_source(File::from_str(&json_content, FileFormat::Json)))
            }
            ConfigSource::String { content, format } => {
                Ok(builder.add_source(File::from_str(&content, format)))
            }
        }
    }
}

/// 获取全局配置管理器实例（单例模式）
pub fn get_global_config_manager() -> Result<Arc<ConfigManager>> {
    {
        let manager = GLOBAL_CONFIG_MANAGER
            .read()
            .map_err(|e| anyhow!("读取全局配置管理器锁失败: {}", e))?;
        if let Some(ref config_manager) = *manager {
            return Ok(Arc::clone(config_manager));
        }
    }
    {
        let mut manager = GLOBAL_CONFIG_MANAGER
            .write()
            .map_err(|e| anyhow!("获取全局配置管理器写锁失败: {}", e))?;
        if manager.is_none() {
            let config_manager =
                Arc::new(ConfigManager::new().map_err(|e| anyhow!("创建配置管理器失败: {}", e))?);
            *manager = Some(Arc::clone(&config_manager));
            Ok(config_manager)
        } else {
            Ok(Arc::clone(manager.as_ref().unwrap()))
        }
    }
}

/// 全局配置获取函数（使用单例）
#[allow(dead_code)]
pub fn get_config<T: DeserializeOwned>(key: &str) -> Result<T> {
    let manager = get_global_config_manager()?;
    manager.get(key)
}

/// 安全的全局配置获取函数
#[allow(dead_code)]
pub fn get_config_safe<T: DeserializeOwned>(key: &str) -> std::result::Result<T, ConfigError> {
    let manager = get_global_config_manager().map_err(|e| ConfigError::InitializationError {
        message: e.to_string(),
    })?;
    manager.get_safe(key)
}

/// 简化的缓存配置获取（使用内存缓存基本类型）
#[allow(dead_code)]
pub fn get_config_cached_simple(key: &str) -> Result<serde_json::Value> {
    let cache_key = key.to_string();
    {
        let cache = CONFIG_CACHE
            .read()
            .map_err(|e| anyhow!("读取配置缓存锁失败: {}", e))?;
        if let Some(config) = cache.get(&cache_key) {
            if let Ok(value) = config.get::<serde_json::Value>(key) {
                return Ok(value);
            }
        }
    }
    let manager = get_global_config_manager()?;
    let value: serde_json::Value = manager.get(key)?;
    let mut cache = CONFIG_CACHE
        .write()
        .map_err(|e| anyhow!("获取配置缓存写锁失败: {}", e))?;
    cache.insert(cache_key, Arc::new(manager.config.clone()));
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{ConfigManager, ConfigSource};
    use config::FileFormat;
    use std::collections::HashMap;

    #[test]
    fn test_config_manager_new() {
        let manager = ConfigManager::new();
        assert!(manager.is_ok());
    }
    #[test]
    fn test_config_from_string() {
        let toml_content = "[server]\nport = 8080".to_string();
        let source = ConfigSource::String {
            content: toml_content,
            format: FileFormat::Toml,
        };
        let manager = ConfigManager::with_sources(vec![source]).unwrap();
        assert_eq!(manager.get::<i64>("server.port").unwrap(), 8080);
    }
    #[test]
    fn test_config_from_memory() {
        let mut map = HashMap::new();
        map.insert(
            "server.host".to_string(),
            serde_json::Value::String("127.0.0.1".to_string()),
        );
        let source = ConfigSource::Memory(map);
        let manager = ConfigManager::with_sources(vec![source]).unwrap();
        assert_eq!(manager.get::<String>("server.host").unwrap(), "127.0.0.1");
    }
}
