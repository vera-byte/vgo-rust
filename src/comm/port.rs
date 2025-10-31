use std::process::Command;

/// 同步检查端口是否可用（通过系统命令）
///
/// # 参数
/// * `port` - 要检查的端口号
///
/// # 返回值
/// * `bool` - 端口是否可用，true 表示可用，false 表示被占用
pub fn is_port_available_sync(port: u16) -> bool {
    // 检测当前操作系统平台
    if cfg!(target_os = "windows") {
        // Windows 平台使用 netstat 命令检查端口
        match Command::new("cmd")
            .args(&["/C", &format!("netstat -ano | findstr :{}", port)])
            .output()
        {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                // 如果端口只处于 TIME_WAIT 状态，则认为端口可用
                result.is_empty() || result.to_lowercase().contains("time_wait")
            }
            Err(_) => {
                // 命令执行失败，认为端口可用
                true
            }
        }
    } else {
        // Linux/Mac 平台使用 lsof 命令检查端口，只检查 LISTEN 状态
        match Command::new("lsof")
            .args(&["-i", &format!(":{}", port), "-sTCP:LISTEN"])
            .output()
        {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                result.is_empty()
            }
            Err(_) => {
                // 命令执行失败，认为端口可用
                true
            }
        }
    }
}

/// 查找可用端口（同步）
///
/// # 参数
/// * `start_port` - 起始端口号
///
/// # 返回值
/// * `u16` - 找到的可用端口号
pub fn available_port(start_port: u16) -> u16 {
    // 检查是否为打包环境（类似 Node.js 的 process.pkg 检查）
    // 在 Rust 中，我们可以通过环境变量或编译时特性来判断
    if !is_packaged_environment() {
        return start_port;
    }

    let mut port = start_port;

    // 在指定范围内查找可用端口
    while port <= 8010 {
        if is_port_available_sync(port) {
            if port != start_port {
                // 使用彩色输出警告端口被占用的情况
                eprintln!(
                    "\x1b[33mPort {} is occupied, using port {}\x1b[0m",
                    start_port, port
                );
            }
            return port;
        }
        port += 1;
    }

    // 如果在范围内没有找到可用端口，返回默认端口
    8001
}

/// 检查是否为打包环境
///
/// # 返回值
/// * `bool` - 是否为打包环境
pub fn is_packaged_environment() -> bool {
    // 可以通过环境变量或编译时特性来判断
    // 这里提供几种实现方式：

    // 方式1: 通过环境变量判断
    std::env::var("PACKAGED").is_ok()

    // 方式2: 通过编译时特性判断（需要在 Cargo.toml 中定义）
    // cfg!(feature = "packaged")

    // 方式3: 通过可执行文件路径判断
    // std::env::current_exe()
    //     .map(|path| path.to_string_lossy().contains("packaged"))
    //     .unwrap_or(false)
}
