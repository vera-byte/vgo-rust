//! 插件开发工具包 / Plugin Development Kit (PDK)
//!
//! 提供类似 Go pdk 的插件开发体验
//! Provides Go pdk-like plugin development experience
//!
//! # 用法 / Usage
//!
//! ```ignore
//! use v::plugin::pdk::*;
//!
//! #[derive(Default, serde::Deserialize)]
//! struct Config {
//!     name: String,
//! }
//!
//! struct AIExample {
//!     config: Config,
//! }
//!
//! impl Plugin for AIExample {
//!     type Config = Config;
//!
//!     fn new() -> Self {
//!         Self { config: Config::default() }
//!     }
//!
//!     fn receive(&mut self, ctx: &mut Context) -> Result<()> {
//!         let content = ctx.get_payload_str("content").unwrap_or_default();
//!         ctx.reply(json!({
//!             "type": 1,
//!             "content": format!("我是{}, 收到您的消息: {}", self.config.name, content)
//!         }))?;
//!         Ok(())
//!     }
//! }
//!
//! v::run_plugin!(AIExample, "wk.plugin.ai-example", version = "0.1.0", priority = 1);
//! ```

use anyhow::Result;
use clap::Parser;
use serde::de::DeserializeOwned;
pub use serde_json::{json, Value};
use tracing::info;

use super::client::{PluginClient, PluginHandler};

// 重新导出事件监听器 / Re-export event listeners
pub use super::events::{AuthEventListener, StorageEventListener};

/// 插件上下文 / Plugin context
///
/// 类似 Go 的 pdk.Context，提供消息处理的上下文信息
/// Similar to Go's pdk.Context, provides context for message handling
pub struct Context {
    /// 事件类型 / Event type
    pub event_type: String,
    /// 原始载荷 / Raw payload
    pub payload: Value,
    /// 响应数据 / Response data
    response: Option<Value>,
}

impl Context {
    /// 创建新的上下文 / Create new context
    pub fn new(event_type: &str, payload: &Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            payload: payload.clone(),
            response: None,
        }
    }

    /// 获取事件类型 / Get event type
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// 获取载荷中的字符串字段 / Get string field from payload
    pub fn get_payload_str(&self, key: &str) -> Option<&str> {
        self.payload.get(key).and_then(|v| v.as_str())
    }

    /// 获取载荷中的整数字段 / Get integer field from payload
    pub fn get_payload_i64(&self, key: &str) -> Option<i64> {
        self.payload.get(key).and_then(|v| v.as_i64())
    }

    /// 获取载荷中的布尔字段 / Get boolean field from payload
    pub fn get_payload_bool(&self, key: &str) -> Option<bool> {
        self.payload.get(key).and_then(|v| v.as_bool())
    }

    /// 获取载荷中的对象字段 / Get object field from payload
    pub fn get_payload_object(&self, key: &str) -> Option<&serde_json::Map<String, Value>> {
        self.payload.get(key).and_then(|v| v.as_object())
    }

    /// 获取载荷中的数组字段 / Get array field from payload
    pub fn get_payload_array(&self, key: &str) -> Option<&Vec<Value>> {
        self.payload.get(key).and_then(|v| v.as_array())
    }

    /// 反序列化载荷为指定类型 / Deserialize payload to specified type
    pub fn parse_payload<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_value(self.payload.clone()).map_err(Into::into)
    }

    /// 回复消息
    pub fn reply(&mut self, data: Value) -> Result<()> {
        self.response = Some(json!({
            "status": "ok",
            "data": data
        }));
        Ok(())
    }

    /// 回复错误 / Reply with error
    pub fn reply_error(&mut self, message: &str) -> Result<()> {
        self.response = Some(json!({
            "status": "error",
            "message": message
        }));
        Ok(())
    }

    /// 获取响应 / Get response
    pub(crate) fn take_response(&mut self) -> Value {
        self.response.take().unwrap_or(json!({ "status": "ok" }))
    }
}

/// 插件 trait / Plugin trait
///
/// 类似 Go 的插件接口，实现此 trait 来创建插件
/// Similar to Go's plugin interface, implement this trait to create a plugin
pub trait Plugin: Sized {
    /// 配置类型 / Config type
    type Config: Default + DeserializeOwned;

    /// 创建插件实例 / Create plugin instance
    fn new() -> Self;

    /// 获取配置引用（可选实现）/ Get config reference (optional)
    fn config(&self) -> Option<&Self::Config> {
        None
    }

    /// 获取可变配置引用（可选实现）/ Get mutable config reference (optional)
    fn config_mut(&mut self) -> Option<&mut Self::Config> {
        None
    }

    /// 收到消息时调用（类似 Go 的 Receive 方法）/ Called when message received (like Go's Receive)
    fn receive(&mut self, ctx: &mut Context) -> Result<()>;

    /// 插件启动时调用（可选）/ Called on plugin start (optional)
    fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 插件停止时调用（可选）/ Called on plugin stop (optional)
    fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// 配置更新时调用（可选）/ Called when config updates (optional)
    fn on_config_update(&mut self, _config: Self::Config) -> Result<()> {
        Ok(())
    }

    /// 声明插件能力（可选）/ Declare plugin capabilities (optional)
    ///
    /// 默认返回空能力，插件需要明确声明所需的能力
    /// Default returns empty capabilities, plugins must explicitly declare required capabilities
    fn capabilities(&self) -> Vec<String> {
        // 默认无能力，插件需要明确申请 / Default no capabilities, plugins must explicitly request
        vec![]
    }
}

/// 插件包装器，将 Plugin trait 适配到 PluginHandler
/// Plugin wrapper, adapts Plugin trait to PluginHandler
struct PluginWrapper<P: Plugin> {
    plugin: P,
    name: &'static str,
    version: &'static str,
    priority: i32,
}

impl<P: Plugin> PluginHandler for PluginWrapper<P> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        // 调用插件的 capabilities 方法 / Call plugin's capabilities method
        self.plugin.capabilities()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn config(&mut self, cfg: &Value) -> Result<()> {
        if let Ok(config) = serde_json::from_value::<P::Config>(cfg.clone()) {
            self.plugin.on_config_update(config)?;
        }
        Ok(())
    }

    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        let mut ctx = Context::new(event_type, payload);
        self.plugin.receive(&mut ctx)?;
        Ok(ctx.take_response())
    }
}

/// 命令行参数 / CLI arguments
#[derive(Parser, Debug)]
#[command(about = "v-connect-im plugin")]
struct PluginArgs {
    /// Unix Socket 路径 / Unix Socket path
    #[arg(long)]
    socket: Option<String>,

    /// 启用 debug 模式 / Enable debug mode
    #[arg(long, short = 'd')]
    debug: bool,

    /// 日志级别 / Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

/// 插件配置 / Plugin configuration
#[derive(serde::Deserialize)]
struct PluginConfig {
    plugin_no: String,
    version: String,
    priority: i32,
}

/// 运行插件服务器 / Run plugin server
///
/// 这是插件的主入口函数，负责：
/// This is the main entry function for plugins, responsible for:
///
/// 1. 读取 plugin.json 配置 / Read plugin.json configuration
/// 2. 解析命令行参数 / Parse command line arguments
/// 3. 初始化日志系统 / Initialize logging system
/// 4. 创建并启动插件客户端 / Create and start plugin client
/// 5. 处理优雅关闭 / Handle graceful shutdown
///
/// # 类型参数 / Type Parameters
///
/// * `P` - 实现了 `Plugin` trait 的插件类型 / Plugin type that implements the `Plugin` trait
///
/// # 示例 / Example
///
/// ```no_run
/// use v::plugin::pdk::{Plugin, run_server};
///
/// struct AIExample;
///
/// impl Plugin for AIExample {
///     // ... implementation
/// }
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     run_server::<AIExample>().await
/// }
/// ```
pub async fn run_server<P: Plugin>() -> Result<()> {
    // 读取 plugin.json 配置 / Read plugin.json configuration
    let config_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("plugin.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("plugin.json"));

    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
        anyhow::anyhow!("Failed to read plugin.json: {}. Path: {:?}", e, config_path)
    })?;

    let config: PluginConfig = serde_json::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse plugin.json: {}", e))?;

    let plugin_no = config.plugin_no;
    let version = config.version;
    let priority = config.priority;
    let args = PluginArgs::parse();

    // 初始化日志 / Initialize logging
    let log_level = if args.debug {
        tracing::Level::DEBUG
    } else {
        match args.log_level.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(args.debug) // 在 debug 模式下显示目标模块
        .with_thread_ids(args.debug) // 在 debug 模式下显示线程 ID
        .with_line_number(args.debug) // 在 debug 模式下显示行号
        .init();

    if args.debug {
        info!("🐛 Debug mode enabled");
    }
    info!("📊 Log level: {:?}", log_level);

    // 从插件编号提取名称 / Extract name from plugin number
    let name = plugin_no
        .strip_prefix("wk.plugin.")
        .or_else(|| plugin_no.strip_prefix("v.plugin."))
        .unwrap_or(&plugin_no);

    let socket_path = args
        .socket
        .unwrap_or_else(|| format!("./plugins/{}.sock", name));

    info!(
        "🚀 {} v{} starting... (priority: {})",
        plugin_no, version, priority
    );
    info!("📡 Socket path: {}", socket_path);

    let plugin = P::new();
    let wrapper = PluginWrapper {
        plugin,
        name: Box::leak(plugin_no.into_boxed_str()),
        version: Box::leak(version.into_boxed_str()),
        priority,
    };

    let mut client = PluginClient::new(socket_path, wrapper);
    client.run_forever_with_ctrlc().await
}
