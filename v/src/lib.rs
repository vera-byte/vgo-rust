// v 库主入口，按需导出模块

pub mod comm;
pub use crate::comm::config::*;

pub mod db;
pub use crate::db::database::*;

// 导出通用仓库 Trait
pub mod repo;
pub use crate::repo::*;

// 重新导出属性宏，允许使用 #[v::base_model]
pub use v_macros::base_model;
