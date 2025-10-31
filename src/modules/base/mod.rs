/// Base 模块
/// 包含基础功能和通用组件

pub mod models;
pub mod routes;
// pub mod middleware; // 暂时注释掉，避免未使用警告
// pub mod service; // 暂时注释掉，避免未使用警告

// 重新导出主要的公共接口
// pub use models::*;

/// 注册base模块的路由
pub fn register_base_routes() {
    // 这里可以添加路由注册逻辑
    println!("Base routes registered");
}

/// 注册base模块的命令
pub fn register_base_commands() {
    // 这里可以添加命令注册逻辑
    println!("Base commands registered");
}