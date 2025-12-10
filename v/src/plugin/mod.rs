//! # 插件系统 / Plugin System
//!
//! 专用插件系统，完全使用 Protobuf 通信
//! Specialized plugin system, fully using Protobuf communication
//!
//! 支持的插件类型：
//! Supported plugin types:
//! - 认证插件 (Auth Plugin): 实现 `AuthEventListener`
//! - 存储插件 (Storage Plugin): 实现 `StorageEventListener`

pub mod client;
pub mod events;
pub mod installer;
pub mod pdk;
#[cfg(feature = "protobuf")]
pub mod proto;
pub mod protocol;
pub mod types;
