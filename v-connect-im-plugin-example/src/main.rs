//! v-connect-im 示例插件 / v-connect-im Example Plugin
//!
//! 这是一个示例插件，演示如何创建和运行 v-connect-im 插件
//! This is an example plugin demonstrating how to create and run v-connect-im plugins

use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use tracing::info;
use v::plugin::client::{PluginClient, PluginHandler};

/// 插件命令行参数 / Plugin command line arguments
#[derive(Parser, Debug)]
#[command(name = "example")]
#[command(about = "v-connect-im example plugin", long_about = None)]
struct Args {
    /// Unix Socket 路径 / Unix Socket path
    #[arg(long, default_value = "./plugins/example.sock")]
    socket: String,
}

/// 插件主函数 / Plugin main function
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志 / Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();
    info!("🚀 v-connect-im Example Plugin starting...");
    info!("📡 Socket path: {}", args.socket);
    let handler = ExamplePlugin::new(None);
    let mut client = PluginClient::new(args.socket, handler);
    client.run_forever_with_ctrlc().await
}

struct ExamplePlugin;

impl ExamplePlugin {
    fn new(_config: Option<Value>) -> Self {
        Self
    }
}

impl PluginHandler for ExamplePlugin {
    fn name(&self) -> &'static str { "example" }
    fn version(&self) -> &'static str { "0.1.0" }
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        let _ = (event_type, payload);
        Ok(serde_json::json!({ "status": "ok", "processed": true }))
    }
}
