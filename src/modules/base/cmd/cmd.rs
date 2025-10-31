use clap::{Arg, Command};
use crate::command_registry::CommandModule;

/// Base模块的命令处理器
pub struct BaseCommands;

impl CommandModule for BaseCommands {
    fn module_name(&self) -> &'static str {
        "base"
    }
    
    fn register_commands(&self) -> Vec<Command> {
        vec![
            Command::new("info")
                .about("显示基础信息")
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("显示详细信息")
                        .action(clap::ArgAction::SetTrue),
                ),
            Command::new("status")
                .about("显示系统状态")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .value_name("FORMAT")
                        .help("输出格式 (json|text)")
                        .default_value("text"),
                ),
        ]
    }
    
    fn handle_command(&self, command_name: &str, matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match command_name {
            "info" => {
                let verbose = matches.get_flag("verbose");
                if verbose {
                    println!("vgo-rust 基础模块详细信息:");
                    println!("  版本: {}", env!("CARGO_PKG_VERSION"));
                    println!("  作者: 金书记");
                    println!("  描述: 基于 Rust 的 Web 应用服务器");
                    println!("  模块: base");
                } else {
                    println!("vgo-rust base 模块 v{}", env!("CARGO_PKG_VERSION"));
                }
            }
            "status" => {
                let format = matches.get_one::<String>("format").unwrap();
                match format.as_str() {
                    "json" => {
                        println!(r#"{{"status": "running", "module": "base", "version": "{}"}}"#, env!("CARGO_PKG_VERSION"));
                    }
                    "text" => {
                        println!("Base模块状态: 运行中");
                        println!("版本: {}", env!("CARGO_PKG_VERSION"));
                    }
                    _ => {
                        return Err(format!("不支持的格式: {}", format).into());
                    }
                }
            }
            _ => {
                return Err(format!("未知命令: {}", command_name).into());
            }
        }
        Ok(())
    }
}

/// 注册Base模块的命令
pub fn register_base_commands() {
    crate::command_registry::register_module(Box::new(BaseCommands));
}