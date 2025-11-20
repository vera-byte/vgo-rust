use std::ptr;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("============================");
    println!("v::get_config");
    println!(
        "获取 port 配置值: {:?}",
        v::get_config::<String>("port").unwrap_or("3306".to_string())
    );
    println!("============================");

    // 通过 v crate 的公共接口获取全局配置管理器
    let manager = v::get_global_config_manager()?;
    let manager2 = v::get_global_config_manager()?;
    println!("============================");
    println!("New两次,判断是否为同一个实例");
    println!("{:?}", ptr::eq(manager.as_ref(), manager2.as_ref()));
    println!("============================");

    println!("============================");
    println!("获取所有配置");
    println!("{:?}", manager.get_all());
    println!("============================");
    let host = manager
        .get_string("server.host")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = manager.get::<u16>("server.port").unwrap_or(8080);
    let debug = manager.get::<bool>("server.debug").unwrap_or(false);

    println!("server: {}:{} (debug={})", host, port, debug);

    match v::get_config_safe::<String>("logging.level") {
        Ok(level) => println!("logging.level: {}", level),
        Err(e) => println!("logging.level 获取失败: {}", e),
    }

    // 打印配置源信息，确认加载顺序与来源
    manager.print_sources_info();

    Ok(())
}
