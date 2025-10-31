pub mod auth;
pub mod error;
pub mod middleware;
pub mod api;
#[path = "bootstrap/app_bootstrap.rs"]
pub mod app_bootstrap;
#[path = "bootstrap/command_registry.rs"]
pub mod command_registry;
#[path = "bootstrap/route_registry.rs"]
pub mod route_registry;
pub mod comm;
pub mod conf;
pub mod stp_util_demo;

// Modules
pub mod modules;
pub mod schema;

/// 初始化所有模块的命令
pub fn init_commands() {
    // 注册base模块的命令
    modules::base::register_base_commands();

    // 这里可以添加其他模块的命令注册
    // modules::space::register_space_commands();
}

/// 初始化所有模块的路由
pub fn init_routes() {
    // 注册base模块的路由
    modules::base::register_base_routes();

    // 这里可以添加其他模块的路由注册
    // modules::space::register_space_routes();
}

// Re-export bootstrap modules
pub use app_bootstrap::*;
pub use command_registry::*;
pub use route_registry::*;
