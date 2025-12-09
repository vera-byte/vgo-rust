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
// 插件主结构 / Plugin Main Structure
// ============================================================================
// 注意：插件元信息（PLUGIN_NO、VERSION、PRIORITY）现在从 plugin.json 读取
// Note: Plugin metadata (PLUGIN_NO, VERSION, PRIORITY) is now read from plugin.json

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
        // TODO: 暴露配置访问方法
        None
    }

    /// 获取配置可变引用 / Get mutable configuration reference
    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        // TODO: 暴露配置访问方法
        None
    }

    /// 配置更新回调 / Configuration update callback
    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        info!("📝 配置已更新 / Config updated: {:?}", config);
        // TODO: 实现配置更新逻辑
        Ok(())
    }

    /// 接收并处理存储事件 / Receive and handle storage events
    ///
    /// 使用 PDK 提供的自动事件分发功能
    /// Use PDK's auto event dispatch feature
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // ✅ 使用 PDK 的自动分发函数
        // 注意：这里需要从 Context 获取 EventMessage
        // TODO: 需要更新 Context 以暴露 EventMessage
        v::warn!("⚠️  等待 Context 更新以支持 EventMessage / Waiting for Context update");
        Ok(())
    }
}

// ============================================================================
// 程序入口 / Program Entry Point
// ============================================================================

/// 存储插件程序入口点 / Storage plugin program entry point
#[tokio::main]
async fn main() -> Result<()> {
    // 启动存储插件服务器 / Start storage plugin server
    // 插件元信息从 plugin.json 自动读取 / Plugin metadata is automatically read from plugin.json
    v::plugin::pdk::run_server::<StoragePlugin>().await
}
