use regex::Regex;
use std::env;
use std::fs;
use std::path::PathBuf;

/// 获得配置文件中的 keys
/// 从配置文件中提取密钥字符串
pub fn get_keys() -> Result<String, Box<dyn std::error::Error>> {
    // 获取当前可执行文件的目录
    let current_dir = env::current_exe()?
        .parent()
        .ok_or("无法获取当前目录")?
        .to_path_buf();

    // 构建配置文件路径
    let config_file = current_dir.join("../config/config.default.js");

    // 读取配置文件内容
    let config_content = fs::read_to_string(config_file)?;

    // 使用正则表达式提取 keys
    let re = Regex::new(r"keys: '([^']+)'")?;
    if let Some(captures) = re.captures(&config_content) {
        if let Some(keys) = captures.get(1) {
            return Ok(keys.as_str().to_string());
        }
    }

    Err("无法找到配置文件中的 keys".into())
}

/// 项目数据目录
/// 创建并返回项目的主数据目录路径
#[allow(dead_code)]
pub fn p_data_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 获取用户主目录
    let home_dir = dirs::home_dir().ok_or("无法获取用户主目录")?;

    // 使用简单的目录名而不是MD5哈希
    let keys = get_keys().unwrap_or_else(|_| "default".to_string());
    let dir_name = format!("cool-admin-{}", keys.chars().take(8).collect::<String>());

    // 构建目录路径
    let dir_path = home_dir.join(".cool-admin").join(dir_name);

    // 如果目录不存在则创建
    if !dir_path.exists() {
        fs::create_dir_all(&dir_path)?;
    }

    Ok(dir_path)
}

/// 上传目录
/// 创建并返回文件上传目录路径
#[allow(dead_code)]
pub fn p_upload_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_path = p_data_path()?;
    let upload_path = data_path.join("upload");

    // 如果目录不存在则创建
    if !upload_path.exists() {
        fs::create_dir_all(&upload_path)?;
    }

    Ok(upload_path)
}

/// 插件目录
/// 创建并返回插件存储目录路径
#[allow(dead_code)]
pub fn p_plugin_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_path = p_data_path()?;
    let plugin_path = data_path.join("plugin");

    // 如果目录不存在则创建
    if !plugin_path.exists() {
        fs::create_dir_all(&plugin_path)?;
    }

    Ok(plugin_path)
}

/// SQLite 数据库文件路径
/// 返回 SQLite 数据库文件的完整路径
#[allow(dead_code)]
pub fn p_sqlite_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_path = p_data_path()?;
    Ok(data_path.join("cool.sqlite"))
}

/// 缓存目录
/// 返回缓存文件存储目录路径
#[allow(dead_code)]
pub fn p_cache_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_path = p_data_path()?;
    Ok(data_path.join("cache"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_functions() {
        // 测试各个路径函数是否能正常工作
        assert!(p_data_path().is_ok());
        assert!(p_upload_path().is_ok());
        assert!(p_plugin_path().is_ok());
        assert!(p_sqlite_path().is_ok());
        assert!(p_cache_path().is_ok());
    }
}
