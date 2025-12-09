//! # v-connect-im 存储插件 / v-connect-im Storage Plugin
//!
//! 基于 Sled 实现的高性能存储插件
//! High-performance storage plugin based on Sled
//!
//! ## 功能特性 / Features
//!
//! - ✅ 消息持久化 / Message persistence
//! - ✅ 离线消息管理 / Offline message management
//! - ✅ 房间成员管理 / Room member management
//! - ✅ 已读回执存储 / Read receipt storage
//! - ✅ 高性能嵌入式数据库 / High-performance embedded database
//!
//! ## 设计模式 / Design Pattern
//!
//! 本插件采用优雅的 Trait 事件监听器模式：
//! This plugin uses elegant Trait-based event listener pattern:
//!
//! - 使用 `v::plugin::pdk::StorageEventListener` trait 抽象存储行为 / Use `v::plugin::pdk::StorageEventListener` trait to abstract storage behavior
//! - 实现 `SledStorageEventListener` 具体存储逻辑 / Implement `SledStorageEventListener` for concrete storage logic
//! - 通过 trait 内置的 `dispatch()` 方法自动分发事件 / Auto dispatch events via trait's built-in `dispatch()` method
//! - 清晰的职责分离，零样板代码 / Clear separation of concerns, zero boilerplate code

// ============================================================================
// 模块声明 / Module Declarations
// ============================================================================

mod sled_listener;

// ============================================================================
// 依赖导入 / Dependencies
// ============================================================================

use anyhow::Result;
use v::info;
use v::plugin::pdk::run_storage_server;

use sled_listener::{SledStorageConfig, SledStorageEventListener};

// ============================================================================
// 注意：插件元信息（PLUGIN_NO、VERSION、PRIORITY）现在从 plugin.json 读取
// Note: Plugin metadata (PLUGIN_NO, VERSION, PRIORITY) is now read from plugin.json

// ============================================================================
// 注意：不再需要 StoragePlugin 结构和 Plugin trait 实现
// Note: No longer need StoragePlugin struct and Plugin trait implementation
// 直接使用 SledStorageEventListener + run_storage_server
// Directly use SledStorageEventListener + run_storage_server
// ============================================================================

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 存储插件程序入口点 / Storage plugin program entry point
///
/// 使用新的 run_storage_server 函数，不需要实现 Plugin trait
/// Use new run_storage_server function, no need to implement Plugin trait
#[tokio::main]
async fn main() -> Result<()> {
    info!("🗄️  启动存储插件 / Starting Storage Plugin");

    // 使用专门的存储插件运行器 / Use dedicated storage plugin runner
    // 不需要 Plugin trait 和 Context，直接使用 StorageEventListener
    // No need for Plugin trait and Context, directly use StorageEventListener
    run_storage_server::<SledStorageEventListener, SledStorageConfig, _>(|config| {
        info!("📝 使用配置 / Using config: {:?}", config);

        // 验证配置 / Validate configuration
        config.validate()?;

        // 创建监听器 / Create listener
        SledStorageEventListener::new(config)
    })
    .await
}
