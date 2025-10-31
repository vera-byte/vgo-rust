use clap::ArgMatches;
use std::error::Error;

mod auth;
mod error;
mod middleware;
#[path = "bootstrap/app_bootstrap.rs"]
mod app_bootstrap;
#[path = "bootstrap/command_registry.rs"]
mod command_registry;
#[path = "bootstrap/route_registry.rs"]
mod route_registry;
mod cmd {
    #[path = "../cmd/version.rs"]
    pub mod version;
    pub use version::*;
}
mod comm;
mod conf;
mod schema;


// Modules
mod modules;

/// 初始化所有模块的命令
fn init_commands() {
    // 注册base模块的命令
    modules::base::register_base_commands();

    // 这里可以添加其他模块的命令注册
    // modules::space::register_space_commands();
}

/// 初始化所有模块的路由
fn init_routes() {
    // 注册base模块的路由
    modules::base::register_base_routes();

    // 这里可以添加其他模块的路由注册
    // modules::space::register_space_routes();
}

use comm::enhanced_config::EnhancedConfigManager;
use app_bootstrap::{AppBootstrap, AppConfig};
use command_registry::{build_app, handle_command};
use cmd::handle_version_command;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化所有模块的命令
    init_commands();

    // 构建命令行应用
    let matches: ArgMatches = build_app().get_matches();

    match matches.subcommand() {
        Some(("server", sub_matches)) => {
            handle_server_command(sub_matches).await?;
        }
        Some(("version", _)) => {
            handle_version_command();
        }
        Some((command_name, sub_matches)) => {
            // 尝试使用模块处理命令
            if let Err(e) = handle_command(command_name, sub_matches) {
                eprintln!("处理命令 '{}' 时出错: {}", command_name, e);
                std::process::exit(1);
            }
        }
        _ => {
            // 这种情况不应该发生，因为我们设置了 subcommand_required(true)
            eprintln!("未知命令，请使用 --help 查看可用命令");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn handle_server_command(_matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    // 初始化路由
    init_routes();

    // 创建并初始化增强的配置管理器
    let config_manager = EnhancedConfigManager::new().await?;

    // 验证必需的配置
    config_manager.validate_required_config()?;

    // 打印配置摘要
    config_manager.print_config_summary();

    // 从配置管理器获取应用配置
    let app_config = config_manager.get_app_config();

    // 创建应用配置
    let config = AppConfig {
        host: app_config.server_host.clone(),
        port: app_config.server_port,
        workers: app_config.server_workers,
        debug: app_config.server_debug,
    };

    // 启动应用
    AppBootstrap::new().with_config(config).run().await?;

    Ok(())
}
