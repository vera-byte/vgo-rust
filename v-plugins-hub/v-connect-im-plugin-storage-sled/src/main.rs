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
use v::plugin::pdk::{Context, Plugin, StorageEventListener};

use sled_listener::{SledStorageConfig, SledStorageEventListener};

// ============================================================================
// 插件元信息 / Plugin Metadata
// ============================================================================

/// 插件唯一标识符 / Plugin unique identifier
const PLUGIN_NO: &str = "v.plugin.storage-sled";

/// 插件版本号 / Plugin version
const VERSION: &str = "0.1.0";

/// 插件优先级 / Plugin priority
/// 存储插件应该有较高优先级以确保数据及时保存
/// Storage plugin should have high priority to ensure data is saved promptly
const PRIORITY: i32 = 900;

// ============================================================================
// 插件主结构 / Plugin Main Structure
// ============================================================================

/// 存储插件主结构 / Storage plugin main structure
///
/// 使用组合模式，将具体的存储实现委托给 `SledStorageEventListener`
/// Uses composition pattern, delegates concrete storage implementation to `SledStorageEventListener`
struct StoragePlugin {
    /// 存储事件监听器 / Storage event listener
    listener: SledStorageEventListener,
}

impl Plugin for StoragePlugin {
    type Config = SledStorageConfig;

    /// 创建新的存储插件实例 / Create new storage plugin instance
    fn new() -> Self {
        info!("🗄️  初始化存储插件 / Initializing Storage Plugin");

        let config = SledStorageConfig::default();
        let listener = SledStorageEventListener::new(config)
            .expect("无法创建存储监听器 / Failed to create storage listener");

        info!("✅ 存储插件初始化完成 / Storage Plugin initialized");

        Self { listener }
    }

    /// 获取配置引用 / Get configuration reference
    fn config(&self) -> Option<&Self::Config> {
        Some(&self.listener.config)
    }

    /// 获取配置可变引用 / Get mutable configuration reference
    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        Some(self.listener.config_mut())
    }

    /// 配置更新回调 / Configuration update callback
    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        info!("📝 配置已更新 / Config updated: {:?}", config);

        // 如果数据库路径改变，需要重新打开数据库
        // If database path changed, need to reopen database
        if config.db_path != self.listener.config.db_path {
            v::warn!("⚠️  数据库路径已改变，需要重启插件 / Database path changed, plugin restart required");
        }

        *self.listener.config_mut() = config;
        Ok(())
    }

    /// 声明插件能力 / Declare plugin capabilities
    ///
    /// 存储插件声明 "storage" 能力，服务器会将 storage.* 事件路由到此插件
    /// Storage plugin declares "storage" capability, server routes storage.* events to this plugin
    fn capabilities(&self) -> Vec<String> {
        vec!["storage".into()]
    }

    /// 接收并处理存储事件 / Receive and handle storage events
    ///
    /// 使用优雅的 trait 事件监听器模式进行分发
    /// Use elegant trait-based event listener pattern for dispatch
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // 使用 tokio 运行时执行异步方法，调用 trait 的自动分发方法
        // Use tokio runtime to execute async method, call trait's auto dispatch method
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.listener.dispatch(ctx))
        })
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 存储插件程序入口点 / Storage plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 启动存储插件服务器 / Start storage plugin server
    v::plugin::pdk::run_server::<StoragePlugin>(PLUGIN_NO, VERSION, PRIORITY).await
}
