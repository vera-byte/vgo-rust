//! 插件开发工具包 / Plugin Development Kit (PDK)

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
}

/// 插件包装器，将 Plugin trait 适配到 PluginHandler
/// Plugin wrapper, adapts Plugin trait to PluginHandler
struct PluginWrapper<P: Plugin> {
    plugin: P,
    name: &'static str,
    version: &'static str,
    priority: i32,
    capabilities: Vec<String>,
    protocol: crate::plugin::protocol::ProtocolFormat,
}

impl<P: Plugin> PluginHandler for PluginWrapper<P> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        // 从配置文件读取的能力列表 / Capabilities list read from config file
        self.capabilities.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn config(&mut self, cfg: &str) -> Result<()> {
        if !cfg.is_empty() {
            if let Ok(value) = serde_json::from_str::<Value>(cfg) {
                if let Ok(config) = serde_json::from_value::<P::Config>(value) {
                    self.plugin.on_config_update(config)?;
                }
            }
        }
        Ok(())
    }

    fn on_event(
        &mut self,
        event: &crate::plugin::protocol::EventMessage,
    ) -> Result<crate::plugin::protocol::EventResponse> {
        // 从 payload 解析为 JSON Value（临时兼容）
        let payload: Value = if event.payload.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&event.payload)?
        };

        let mut ctx = Context::new(&event.event_type, &payload);
        self.plugin.receive(&mut ctx)?;
        let response_data = ctx.take_response();

        // 构建 EventResponse
        Ok(crate::plugin::protocol::EventResponse {
            status: "ok".to_string(),
            flow: "continue".to_string(),
            data: serde_json::to_vec(&response_data)?,
            error: String::new(),
        })
    }

    // 使用配置文件中指定的协议 / Use protocol specified in config file
    fn protocol(&self) -> crate::plugin::protocol::ProtocolFormat {
        self.protocol
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
    #[serde(default)]
    capabilities: Vec<String>,
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

    eprintln!("🔍 Reading plugin.json from: {:?}", config_path);

    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
        anyhow::anyhow!("Failed to read plugin.json: {}. Path: {:?}", e, config_path)
    })?;

    eprintln!("📄 plugin.json content:\n{}", config_content);

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

    // 使用 Protobuf 协议 / Use Protobuf protocol
    let protocol = crate::plugin::protocol::ProtocolFormat::Protobuf;

    info!(
        "🚀 {} v{} starting... (priority: {}, protocol: {:?})",
        plugin_no, version, priority, protocol
    );
    info!("📡 Socket path: {}", socket_path);

    let plugin = P::new();
    let wrapper = PluginWrapper {
        plugin,
        name: Box::leak(plugin_no.into_boxed_str()),
        version: Box::leak(version.into_boxed_str()),
        priority,
        capabilities: config.capabilities,
        protocol,
    };

    let mut client = PluginClient::new(socket_path, wrapper);
    client.run_forever_with_ctrlc().await
}

// ============================================================================
// 自动事件分发 / Auto Event Dispatch
// ============================================================================

/// 分发存储事件到对应的监听器方法 / Dispatch storage event to listener method
///
/// 自动解码 Protobuf 消息并调用对应的方法
/// Automatically decodes Protobuf message and calls corresponding method
pub async fn dispatch_storage_event(
    listener: &mut dyn StorageEventListener,
    event: &crate::plugin::protocol::EventMessage,
) -> Result<crate::plugin::protocol::EventResponse> {
    use crate::plugin::protocol::*;
    use prost::Message;

    match event.event_type.as_str() {
        "storage.message.save" => {
            let req = SaveMessageRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_message_save(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.save" => {
            let req = SaveOfflineMessageRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_save(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.pull" => {
            let req = PullOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_pull(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.ack" => {
            let req = AckOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_ack(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.offline.count" => {
            let req = CountOfflineMessagesRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_offline_count(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.add_member" => {
            let req = AddRoomMemberRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_add_member(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.remove_member" => {
            let req = RemoveRoomMemberRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_remove_member(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "storage.room.list_members" => {
            let req = GetRoomMembersRequest::decode(event.payload.as_slice())?;
            let resp = listener.storage_room_list_members(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        _ => Err(anyhow::anyhow!(
            "Unknown storage event: {}",
            event.event_type
        )),
    }
}

// ============================================================================
// 通用插件运行器 / Generic Plugin Runner
// ============================================================================

/// 插件元数据 / Plugin metadata
struct PluginMetadata {
    plugin_no: String,
    version: String,
    priority: i32,
    capabilities: Vec<String>,
    socket_path: String,
    protocol: crate::plugin::protocol::ProtocolFormat,
}

/// 初始化插件运行环境 / Initialize plugin runtime environment
fn init_plugin_runtime() -> Result<PluginMetadata> {
    // 读取 plugin.json 配置 / Read plugin.json configuration
    let config_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("plugin.json")))
        .unwrap_or_else(|| std::path::PathBuf::from("plugin.json"));

    eprintln!("🔍 Reading plugin.json from: {:?}", config_path);

    let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
        anyhow::anyhow!("Failed to read plugin.json: {}. Path: {:?}", e, config_path)
    })?;

    eprintln!("📄 plugin.json content:\n{}", config_content);

    let plugin_config: PluginConfig = serde_json::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse plugin.json: {}", e))?;

    let plugin_no = plugin_config.plugin_no;
    let version = plugin_config.version;
    let priority = plugin_config.priority;
    let capabilities = plugin_config.capabilities;
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
        .with_target(args.debug)
        .with_thread_ids(args.debug)
        .with_line_number(args.debug)
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

    let protocol = crate::plugin::protocol::ProtocolFormat::Protobuf;

    info!(
        "🚀 {} v{} starting... (priority: {}, protocol: {:?})",
        plugin_no, version, priority, protocol
    );
    info!("📡 Socket path: {}", socket_path);

    Ok(PluginMetadata {
        plugin_no,
        version,
        priority,
        capabilities,
        socket_path,
        protocol,
    })
}

// ============================================================================
// 存储插件专用运行器 / Storage Plugin Runner
// ============================================================================

/// 运行存储插件服务器 / Run storage plugin server
///
/// 专门为 StorageEventListener 设计的运行函数，不需要实现 Plugin trait
/// Dedicated runner for StorageEventListener, no need to implement Plugin trait
///
/// # 类型参数 / Type Parameters
///
/// * `L` - 实现了 `StorageEventListener` trait 的监听器类型
/// * `C` - 配置类型，必须实现 Default 和 DeserializeOwned
///
/// # 示例 / Example
///
/// ```no_run
/// use v::plugin::pdk::{StorageEventListener, run_storage_server};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     run_storage_server::<MyStorageListener, MyConfig>(
///         |config| MyStorageListener::new(config)
///     ).await
/// }
/// ```
pub async fn run_storage_server<L, C, F>(create_listener: F) -> Result<()>
where
    L: StorageEventListener + 'static,
    C: Default + DeserializeOwned,
    F: FnOnce(C) -> Result<L>,
{
    let metadata = init_plugin_runtime()?;

    // 创建监听器 / Create listener
    let user_config = C::default();
    let listener = create_listener(user_config)?;

    let wrapper = StoragePluginWrapper {
        listener: Box::new(listener),
        name: Box::leak(metadata.plugin_no.into_boxed_str()),
        version: Box::leak(metadata.version.into_boxed_str()),
        priority: metadata.priority,
        capabilities: metadata.capabilities,
        protocol: metadata.protocol,
    };

    let mut client = PluginClient::new(metadata.socket_path, wrapper);
    client.run_forever_with_ctrlc().await
}

/// 存储插件包装器 / Storage plugin wrapper
struct StoragePluginWrapper {
    listener: Box<dyn StorageEventListener>,
    name: &'static str,
    version: &'static str,
    priority: i32,
    capabilities: Vec<String>,
    protocol: crate::plugin::protocol::ProtocolFormat,
}

impl PluginHandler for StoragePluginWrapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn config(&mut self, _cfg: &str) -> Result<()> {
        // 存储插件的配置通过构造函数传递，这里不处理
        // Storage plugin config is passed via constructor, not handled here
        Ok(())
    }

    fn on_event(
        &mut self,
        event: &crate::plugin::protocol::EventMessage,
    ) -> Result<crate::plugin::protocol::EventResponse> {
        // 使用 tokio 的 block_in_place 在同步上下文中运行异步代码
        // Use tokio's block_in_place to run async code in sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(dispatch_storage_event(&mut *self.listener, event))
        })
    }

    fn protocol(&self) -> crate::plugin::protocol::ProtocolFormat {
        self.protocol
    }
}

// ============================================================================
// 认证插件专用运行器 / Auth Plugin Runner
// ============================================================================

/// 运行认证插件服务器 / Run auth plugin server
///
/// 专门为 AuthEventListener 设计的运行函数，不需要实现 Plugin trait
/// Dedicated runner for AuthEventListener, no need to implement Plugin trait
///
/// # 类型参数 / Type Parameters
///
/// * `L` - 实现了 `AuthEventListener` trait 的监听器类型
/// * `C` - 配置类型，必须实现 Default 和 DeserializeOwned
///
/// # 示例 / Example
///
/// ```no_run
/// use v::plugin::pdk::{AuthEventListener, run_auth_server};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     run_auth_server::<MyAuthListener, MyConfig>(
///         |config| MyAuthListener::new(config)
///     ).await
/// }
/// ```
pub async fn run_auth_server<L, C, F>(create_listener: F) -> Result<()>
where
    L: AuthEventListener + 'static,
    C: Default + DeserializeOwned,
    F: FnOnce(C) -> Result<L>,
{
    let metadata = init_plugin_runtime()?;

    // 创建监听器 / Create listener
    let user_config = C::default();
    let listener = create_listener(user_config)?;

    let wrapper = AuthPluginWrapper {
        listener: Box::new(listener),
        name: Box::leak(metadata.plugin_no.into_boxed_str()),
        version: Box::leak(metadata.version.into_boxed_str()),
        priority: metadata.priority,
        capabilities: metadata.capabilities,
        protocol: metadata.protocol,
    };

    let mut client = PluginClient::new(metadata.socket_path, wrapper);
    client.run_forever_with_ctrlc().await
}

/// 认证插件包装器 / Auth plugin wrapper
struct AuthPluginWrapper {
    listener: Box<dyn AuthEventListener>,
    name: &'static str,
    version: &'static str,
    priority: i32,
    capabilities: Vec<String>,
    protocol: crate::plugin::protocol::ProtocolFormat,
}

impl PluginHandler for AuthPluginWrapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn version(&self) -> &'static str {
        self.version
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn config(&mut self, _cfg: &str) -> Result<()> {
        // 认证插件的配置通过构造函数传递，这里不处理
        // Auth plugin config is passed via constructor, not handled here
        Ok(())
    }

    fn on_event(
        &mut self,
        event: &crate::plugin::protocol::EventMessage,
    ) -> Result<crate::plugin::protocol::EventResponse> {
        // 使用 tokio 的 block_in_place 在同步上下文中运行异步代码
        // Use tokio's block_in_place to run async code in sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(dispatch_auth_event(&mut *self.listener, event))
        })
    }

    fn protocol(&self) -> crate::plugin::protocol::ProtocolFormat {
        self.protocol
    }
}

/// 分发认证事件到对应的监听器方法 / Dispatch auth event to listener method
pub async fn dispatch_auth_event(
    listener: &mut dyn AuthEventListener,
    event: &crate::plugin::protocol::EventMessage,
) -> Result<crate::plugin::protocol::EventResponse> {
    use crate::plugin::protocol::*;
    use prost::Message;

    match event.event_type.as_str() {
        "auth.login" => {
            let req = LoginRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_login(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "auth.logout" => {
            let req = LogoutRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_logout(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "auth.kick_out" => {
            let req = KickOutRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_kick_out(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "auth.renew_token" => {
            let req = RenewTokenRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_renew_token(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "auth.token_replaced" => {
            let req = TokenReplacedRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_token_replaced(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        "auth.ban_user" => {
            let req = BanUserRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_ban_user(&req).await?;
            Ok(EventResponse {
                status: "ok".to_string(),
                flow: "continue".to_string(),
                data: resp.encode_to_vec(),
                error: String::new(),
            })
        }
        _ => Err(anyhow::anyhow!("Unknown auth event: {}", event.event_type)),
    }
}
