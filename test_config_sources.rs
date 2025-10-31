use config::FileFormat;
use std::collections::HashMap;
use vgo_rust::comm::config::{get_global_config_manager, ConfigManager, ConfigSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试配置数据源功能");
    println!("==================");

    // 测试1: 创建包含多种配置源的ConfigManager
    println!("\n1. 测试多种配置源:");

    let mut memory_config = HashMap::new();
    memory_config.insert("memory_key".to_string(), serde_json::json!("memory_value"));
    memory_config.insert("app_name".to_string(), serde_json::json!("VGO Rust App"));

    let sources = vec![
        ConfigSource::File {
            path: "config.toml".to_string(),
            format: Some(FileFormat::Toml),
            required: false,
        },
        ConfigSource::File {
            path: "nonexistent.toml".to_string(),
            format: Some(FileFormat::Toml),
            required: false,
        },
        ConfigSource::Memory(memory_config),
        ConfigSource::String {
            content: r#"{"string_key": "string_value", "debug": true}"#.to_string(),
            format: FileFormat::Json,
        },
    ];

    let config_manager = ConfigManager::with_sources(sources)?;

    // 打印配置源信息
    config_manager.print_sources_info();

    // 测试2: 获取配置源统计
    println!("\n2. 配置源统计:");
    let (total, loaded, failed) = config_manager.get_sources_stats();
    println!("总计: {}, 成功: {}, 失败: {}", total, loaded, failed);

    // 测试3: 获取活跃配置源
    println!("\n3. 活跃配置源:");
    let active_sources = config_manager.get_active_sources();
    for source in active_sources {
        println!("- {} (优先级: {})", source.source_type, source.priority);
    }

    // 测试4: 获取失败配置源
    println!("\n4. 失败配置源:");
    let failed_sources = config_manager.get_failed_sources();
    for source in failed_sources {
        println!(
            "- {} (优先级: {}): {}",
            source.source_type, source.priority, source.description
        );
    }

    // 测试5: 验证配置值读取
    println!("\n5. 配置值读取测试:");

    // 从内存配置源读取
    if let Ok(memory_value) = config_manager.get_string("memory_key") {
        println!("内存配置源 - memory_key: {}", memory_value);
    }

    // 从字符串配置源读取
    if let Ok(string_value) = config_manager.get_string("string_key") {
        println!("字符串配置源 - string_key: {}", string_value);
    }

    // 从文件配置源读取（如果存在）
    if let Ok(db_host) = config_manager.get_string("database.host") {
        println!("文件配置源 - database.host: {}", db_host);
    }

    // 从环境变量读取（如果设置了APP_TEST_VAR）
    if let Ok(env_value) = config_manager.get_string("test_var") {
        println!("环境变量配置源 - APP_TEST_VAR: {}", env_value);
    }

    // 测试6: 全局配置管理器的配置源信息
    println!("\n6. 全局配置管理器配置源信息:");
    let global_manager = get_global_config_manager()?;
    println!(
        "全局配置管理器配置源数量: {}",
        global_manager.get_sources_info().len()
    );

    println!("\n✅ 配置数据源功能测试完成!");

    Ok(())
}
