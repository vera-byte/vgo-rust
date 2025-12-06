//! # 插件事件监听器 / Plugin Event Listeners
//!
//! 定义各种插件事件监听器 trait
//! Defines various plugin event listener traits

pub mod auth;
pub mod storage;

// 重新导出常用类型 / Re-export common types
pub use auth::AuthEventListener;
pub use storage::StorageEventListener;
