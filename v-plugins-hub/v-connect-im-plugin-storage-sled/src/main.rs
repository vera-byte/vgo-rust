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

mod sled_listener;

use anyhow::Result;
use v::info;
use v::plugin::pdk::run_storage_server;

use sled_listener::{SledStorageConfig, SledStorageEventListener};

#[tokio::main]
async fn main() -> Result<()> {
    info!("🗄️  启动存储插件 / Starting Storage Plugin");
    run_storage_server::<SledStorageEventListener, SledStorageConfig, _>(|config| {
        info!("📝 使用配置 / Using config: {:?}", config);
        // 验证配置 / Validate configuration
        config.validate()?;
        // 创建监听器 / Create listener
        SledStorageEventListener::new(config)
    })
    .await
}
