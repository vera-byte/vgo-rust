/// 模块管理
/// 包含所有业务模块的定义和导出
pub mod base;
pub mod dict;
pub mod space;

// 选择性重新导出，避免命名冲突
// 只导出主要的公共接口，避免全局导出造成的命名冲突
// pub use base::{register_base_commands, register_base_routes}; // 暂时注释掉，避免未使用警告
// pub use dict::controller as dict_controller; // 暂时注释掉，避免未使用警告
// pub use dict::models as dict_models; // 暂时注释掉，避免未使用警告
