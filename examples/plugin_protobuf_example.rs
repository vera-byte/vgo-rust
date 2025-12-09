//! # Protobuf 插件示例 / Protobuf Plugin Example
//!
//! 演示如何使用 Protocol Buffers 协议创建高性能插件
//! Demonstrates how to create high-performance plugins using Protocol Buffers
//!
//! ## 性能优势 / Performance Benefits
//! - ✅ 序列化速度提升 3-10 倍 / 3-10x faster serialization
//! - ✅ 数据体积减少 60-80% / 60-80% smaller data size
//! - ✅ 类型安全 / Type safety
//! - ✅ 向后兼容 / Backward compatibility
//!
//! ## 运行方式 / How to Run
//! ```bash
//! cargo run --example plugin_protobuf_example --features protobuf -- --socket ./plugins/protobuf-demo.sock
//! ```

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use v::plugin::client::{PluginClient, PluginHandler};
use v::plugin::protocol::ProtocolFormat;
use v::{debug, info};

// ============================================================================
// 插件实现 / Plugin Implementation
// ============================================================================

/// Protobuf 演示插件 / Protobuf demo plugin
struct ProtobufDemoPlugin {
    /// 消息计数器 / Message counter
    message_count: u64,
    /// 性能统计 / Performance stats
    stats: HashMap<String, u64>,
}

impl ProtobufDemoPlugin {
    fn new() -> Self {
        info!("🚀 初始化 Protobuf 演示插件 / Initializing Protobuf demo plugin");
        Self {
            message_count: 0,
            stats: HashMap::new(),
        }
    }
}

impl PluginHandler for ProtobufDemoPlugin {
    fn name(&self) -> &'static str {
        "v.plugin.protobuf-demo"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["message".to_string(), "stats".to_string()]
    }

    fn priority(&self) -> i32 {
        500
    }

    /// 指定使用 Protobuf 协议 / Specify Protobuf protocol
    fn protocol(&self) -> ProtocolFormat {
        #[cfg(feature = "protobuf")]
        {
            info!("✅ 使用 Protobuf 协议 / Using Protobuf protocol");
            ProtocolFormat::Protobuf
        }
        #[cfg(not(feature = "protobuf"))]
        {
            info!("⚠️  Protobuf 未启用，回退到 JSON / Protobuf not enabled, falling back to JSON");
            ProtocolFormat::Json
        }
    }

    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        debug!("📨 收到事件 / Received event: {}", event_type);

        match event_type {
            "message.incoming" => {
                self.message_count += 1;
                *self.stats.entry("messages".to_string()).or_insert(0) += 1;

                let message_id = payload
                    .get("message_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                let from_uid = payload
                    .get("from_uid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                info!(
                    "💬 处理消息 #{} / Processing message #{}: {} -> {}",
                    self.message_count, self.message_count, from_uid, message_id
                );

                // 返回处理结果 / Return processing result
                Ok(json!({
                    "status": "ok",
                    "flow": "continue",
                    "data": {
                        "processed": true,
                        "message_count": self.message_count,
                        "protocol": "protobuf"
                    }
                }))
            }

            "stats.get" => {
                info!("📊 返回统计信息 / Returning statistics");

                Ok(json!({
                    "status": "ok",
                    "flow": "continue",
                    "data": {
                        "message_count": self.message_count,
                        "stats": self.stats,
                        "protocol": "protobuf"
                    }
                }))
            }

            _ => {
                debug!("⏭️  未知事件类型 / Unknown event type: {}", event_type);
                Ok(json!({
                    "status": "ok",
                    "flow": "continue",
                    "data": {}
                }))
            }
        }
    }
}

// ============================================================================
// 主函数 / Main Function
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志 / Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🚀 Protobuf Plugin Demo / Protobuf 插件演示");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");

    #[cfg(feature = "protobuf")]
    info!("✅ Protobuf 特性已启用 / Protobuf feature enabled");
    #[cfg(not(feature = "protobuf"))]
    info!("⚠️  Protobuf 特性未启用，将使用 JSON / Protobuf feature not enabled, will use JSON");

    info!("");

    // 解析命令行参数 / Parse command line arguments
    let socket_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./plugins/protobuf-demo.sock".to_string());

    info!("🔌 Socket 路径 / Socket path: {}", socket_path);
    info!("");

    // 创建插件实例 / Create plugin instance
    let handler = ProtobufDemoPlugin::new();

    // 创建客户端 / Create client
    let mut client = PluginClient::new(socket_path, handler);

    info!("🎯 启动插件客户端 / Starting plugin client");
    info!("");

    // 运行客户端 / Run client
    client.run_forever_with_ctrlc().await?;

    info!("");
    info!("👋 插件已停止 / Plugin stopped");
    info!("");

    Ok(())
}
