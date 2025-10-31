use clap::{Arg, Command};
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;

/// 命令注册器特trait，各模块实现此trait来注册命令
pub trait CommandModule {
    /// 获取模块名称
    fn module_name(&self) -> &'static str;
    
    /// 注册模块的子命令
    fn register_commands(&self) -> Vec<Command>;
    
    /// 处理模块命令
    fn handle_command(&self, command_name: &str, matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// 命令注册器，使用单例模式
#[allow(dead_code)]
pub struct CommandRegistry {
    modules: HashMap<String, Box<dyn CommandModule + Send + Sync>>,
}

#[allow(dead_code)]
impl CommandRegistry {
    /// 创建新的命令注册器
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }
    
    /// 获取全局单例实例
    pub fn instance() -> &'static Arc<Mutex<CommandRegistry>> {
        static INSTANCE: OnceLock<Arc<Mutex<CommandRegistry>>> = OnceLock::new();
        INSTANCE.get_or_init(|| Arc::new(Mutex::new(CommandRegistry::new())))
    }
    
    /// 注册模块
    pub fn register_module(&mut self, module: Box<dyn CommandModule + Send + Sync>) {
        let module_name = module.module_name().to_string();
        self.modules.insert(module_name, module);
    }
    
    /// 构建完整的命令行应用
    pub fn build_app(&self) -> Command {
        let mut app = Command::new("vgo-rust")
            .version("1.0.0")
            .author("金书记")
            .about("基于 Rust 的 Web 应用服务器")
            .subcommand_required(true)
            .arg_required_else_help(true);
        
        // 添加内置的server命令
        app = app.subcommand(
            Command::new("server")
                .about("启动 Web 服务器")
                .arg(
                    Arg::new("host")
                        .long("host")
                        .value_name("HOST")
                        .help("设置服务器主机地址")
                        .default_value("0.0.0.0"),
                )
                .arg(
                    Arg::new("port")
                        .short('p')
                        .long("port")
                        .value_name("PORT")
                        .help("设置服务器端口")
                        .default_value("3000"),
                )
                .arg(
                    Arg::new("workers")
                        .short('w')
                        .long("workers")
                        .value_name("WORKERS")
                        .help("设置工作线程数")
                        .default_value("8"),
                )
                .arg(
                    Arg::new("debug")
                        .short('d')
                        .long("debug")
                        .help("启用调试模式")
                        .action(clap::ArgAction::SetTrue),
                ),
        );
        
        // 添加内置的version命令
        app = app.subcommand(
            Command::new("version")
                .about("显示版本信息")
        );
        
        // 添加各模块注册的命令
        for module in self.modules.values() {
            let commands = module.register_commands();
            for command in commands {
                app = app.subcommand(command);
            }
        }
        
        app
    }
    
    /// 处理命令
    pub fn handle_command(&self, command_name: &str, matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 查找对应的模块来处理命令
        for module in self.modules.values() {
            let commands = module.register_commands();
            for command in commands {
                if command.get_name() == command_name {
                    return module.handle_command(command_name, matches);
                }
            }
        }
        
        Err(format!("未找到处理命令 '{}' 的模块", command_name).into())
    }
    
    /// 获取所有注册的模块名称
    pub fn get_registered_modules(&self) -> Vec<&str> {
        self.modules.keys().map(|s| s.as_str()).collect()
    }
}

/// 便捷函数：注册模块
#[allow(dead_code)]
pub fn register_module(module: Box<dyn CommandModule + Send + Sync>) {
    let registry = CommandRegistry::instance();
    let mut registry = registry.lock().unwrap();
    registry.register_module(module);
}

#[allow(dead_code)]
pub fn build_app() -> Command {
    let registry = CommandRegistry::instance();
    let registry = registry.lock().unwrap();
    registry.build_app()
}

#[allow(dead_code)]
pub fn handle_command(command_name: &str, matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let registry = CommandRegistry::instance();
    let registry = registry.lock().unwrap();
    registry.handle_command(command_name, matches)
}