//! 插件开发工具包 / Plugin Development Kit (PDK)
//!
//! 专用插件系统，完全使用 Protobuf 通信
//! Specialized plugin system, fully using Protobuf communication

use anyhow::Result;
use clap::Parser;
use serde::de::DeserializeOwned;
use tracing::info;

use super::client::{PluginClient, PluginHandler};

// 重新导出事件监听器 / Re-export event listeners
pub use super::events::{AuthEventListener, StorageEventListener};

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

// ============================================================================
// 专用插件运行器 / Specialized Plugin Runners
// ============================================================================
//
// 只支持以下类型的插件：
// Only the following plugin types are supported:
// - 存储插件 (Storage Plugin): 使用 run_storage_server
// - 认证插件 (Auth Plugin): 使用 run_auth_server
//
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
    let name = plugin_no.strip_prefix("v.plugin.").unwrap_or(&plugin_no);

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
        "auth.validate_token" => {
            let req = ValidateTokenRequest::decode(event.payload.as_slice())?;
            let resp = listener.auth_validate_token(&req).await?;
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
