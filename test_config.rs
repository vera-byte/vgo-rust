use config::FileFormat;
use std::collections::HashMap;
use vgo_rust::comm::config::{
    get_config_safe, get_global_config_manager, ConfigError, ConfigManager, ConfigSource,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 开始测试配置管理器...");

    // 测试1: 基本配置读取
    test_basic_config().await?;

    // 测试2: 错误处理
    test_error_handling().await?;

    // 测试3: 配置验证
    test_config_validation().await?;

    // 测试4: 全局单例
    test_global_singleton().await?;

    // 测试5: 多种配置源
    test_multiple_sources().await?;

    println!("✅ 所有测试通过！");
    Ok(())
}

async fn test_basic_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 测试1: 基本配置读取");

    let config = get_global_config_manager()?;

    // 测试不同类型的配置读取
    let db_host: String = config.get_or("database.host", "default_host".to_string());
    let db_port: u16 = config.get_or("database.port", 5432);
    let debug: bool = config.get_or("server.debug", false);
    let timeout: f64 = config.get_or("server.timeout", 30.0);

    println!("   数据库主机: {}", db_host);
    println!("   数据库端口: {}", db_port);
    println!("   调试模式: {}", debug);
    println!("   超时时间: {}", timeout);

    // 测试配置项存在性检查
    println!("   database.host 存在: {}", config.exists("database.host"));
    println!(
        "   nonexistent.key 存在: {}",
        config.exists("nonexistent.key")
    );

    Ok(())
}

async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚨 测试2: 错误处理");

    let config = get_global_config_manager()?;

    // 测试不存在的配置项
    match get_config_safe::<String>("nonexistent.key") {
        Ok(_) => println!("   ❌ 应该返回错误"),
        Err(ConfigError::KeyNotFound { key }) => {
            println!("   ✅ 正确捕获KeyNotFound错误: {}", key);
        }
        Err(e) => println!("   ⚠️  其他错误: {:?}", e),
    }

    // 测试类型转换错误
    match config.get_safe::<u32>("database.host") {
        Ok(_) => println!("   ❌ 应该返回类型转换错误"),
        Err(ConfigError::TypeConversionError { key, message }) => {
            println!("   ✅ 正确捕获TypeConversionError: {} - {}", key, message);
        }
        Err(e) => println!("   ⚠️  其他错误: {:?}", e),
    }

    Ok(())
}

async fn test_config_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n✅ 测试3: 配置验证");

    let config = get_global_config_manager()?;

    // 测试必需配置项验证
    let required_keys = vec!["database.host", "server.port"];
    match config.validate_required_keys(&required_keys) {
        Ok(()) => println!("   ✅ 所有必需配置项都存在"),
        Err(e) => println!("   ❌ 配置验证失败: {:?}", e),
    }

    // 测试包含不存在配置项的验证
    let invalid_keys = vec!["database.host", "nonexistent.key"];
    match config.validate_required_keys(&invalid_keys) {
        Ok(()) => println!("   ❌ 应该验证失败"),
        Err(ConfigError::KeyNotFound { key }) => {
            println!("   ✅ 正确检测到缺失的配置项: {}", key);
        }
        Err(e) => println!("   ⚠️  其他错误: {:?}", e),
    }

    Ok(())
}

async fn test_global_singleton() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 测试4: 全局单例");

    let config1 = get_global_config_manager()?;
    let config2 = get_global_config_manager()?;

    // 验证是否是同一个实例
    let ptr1 = config1.as_ref() as *const ConfigManager;
    let ptr2 = config2.as_ref() as *const ConfigManager;

    if ptr1 == ptr2 {
        println!("   ✅ 全局单例工作正常 - 返回相同实例");
    } else {
        println!("   ❌ 全局单例失败 - 返回不同实例");
    }

    Ok(())
}

async fn test_multiple_sources() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📚 测试5: 多种配置源");

    // 创建内存配置源
    let mut memory_config = HashMap::new();
    memory_config.insert(
        "test.memory_key".to_string(),
        serde_json::Value::String("memory_value".to_string()),
    );
    memory_config.insert(
        "test.number".to_string(),
        serde_json::Value::Number(serde_json::Number::from(42)),
    );

    // 创建字符串配置源
    let json_config = r#"
    {
        "test": {
            "string_key": "string_value",
            "boolean": true
        }
    }
    "#;

    let sources = vec![
        ConfigSource::Memory(memory_config),
        ConfigSource::String {
            content: json_config.to_string(),
            format: FileFormat::Json,
        },
    ];

    let config = ConfigManager::with_sources(sources)?;

    // 测试从不同源读取配置
    let memory_value: String = config.get_or("test.memory_key", "default".to_string());
    let string_value: String = config.get_or("test.string_key", "default".to_string());
    let number_value: i32 = config.get_or("test.number", 0);
    let boolean_value: bool = config.get_or("test.boolean", false);

    println!("   内存源配置: {}", memory_value);
    println!("   字符串源配置: {}", string_value);
    println!("   数字配置: {}", number_value);
    println!("   布尔配置: {}", boolean_value);

    // 验证值是否正确
    assert_eq!(memory_value, "memory_value");
    assert_eq!(string_value, "string_value");
    assert_eq!(number_value, 42);
    assert_eq!(boolean_value, true);

    println!("   ✅ 多配置源测试通过");

    Ok(())
}
