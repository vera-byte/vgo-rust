use crate::cluster;
use crate::plugins::auth::{AuthPlugin, DefaultAuthPlugin};
use crate::plugins::bridge::RemotePluginManager;
use crate::plugins::{Plugin, PluginRegistry};
use crate::storage;
use dashmap::{DashMap, DashSet};
use parking_lot::RwLock;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// 客户端连接信息 / Client Connection Information
#[derive(Clone)]
pub struct Connection {
    #[allow(dead_code)]
    pub client_id: String, // 客户端唯一ID（当前未读取）/ Client unique ID (currently not read)
    pub uid: Option<String>,                    // 用户ID / User ID
    pub addr: SocketAddr,                       // 客户端地址 / Client address
    pub sender: mpsc::UnboundedSender<Message>, // 消息发送器 / Message sender
    pub last_heartbeat: Arc<std::sync::Mutex<std::time::Instant>>, // 最后心跳时间 / Last heartbeat time
}

/// 服务端全局状态 / Server Global State
pub struct VConnectIMServer {
    pub connections: Arc<DashMap<String, Connection>>, // 客户端连接 / Client connections
    pub webhook_config: Option<crate::config::WebhookConfigLite>, // Webhook配置 / Webhook configuration
    pub auth_config: Option<crate::config::AuthConfigLite>,       // 鉴权配置 / Auth configuration
    pub auth_plugin: Arc<dyn AuthPlugin>,                         // 授权插件 / Auth plugin
    pub plugin_registry: Arc<PluginRegistry>, // 通用插件注册中心 / Plugin registry
    pub remote_plugins: Arc<RemotePluginManager>, // 远程插件管理 / Remote plugin manager
    pub plugin_config: Arc<RwLock<Value>>,    // 插件配置快照 / Plugin config snapshot
    pub acked_ids: Arc<DashMap<String, DashSet<String>>>, // 已确认消息ID / Acked message IDs per client
    pub node_id: String,                                  // 当前节点ID / Current node ID
    pub directory: Arc<cluster::directory::Directory>,    // 目录服务 / Directory service
    pub broker: cluster::broker::ShardBroker,             // 分片代理 / Shard broker
    pub storage: storage::Storage,                        // 存储 / Storage
    pub raft: Arc<cluster::raft::RaftCluster>,            // Raft集群 / Raft cluster
    pub rooms: Arc<DashMap<String, DashSet<String>>>,     // 房间到UID集合 / Room -> UIDs
    pub uid_clients: Arc<DashMap<String, DashSet<String>>>, // UID到客户端集合 / UID -> client_ids
    pub quic_conn_count: Arc<std::sync::atomic::AtomicUsize>, // QUIC连接数 / QUIC connection count
    pub quic_path_updates: Arc<std::sync::atomic::AtomicUsize>, // QUIC路径更新计数 / QUIC path updates count
    pub quic_stream_sent: Arc<std::sync::atomic::AtomicUsize>, // QUIC stream发送计数 / QUIC stream sent count
    pub quic_dgram_sent: Arc<std::sync::atomic::AtomicUsize>, // QUIC datagram发送计数 / QUIC dgram sent count
    pub quic_stream_recv: Arc<std::sync::atomic::AtomicUsize>, // QUIC stream接收计数 / QUIC stream recv count
    pub quic_dgram_recv: Arc<std::sync::atomic::AtomicUsize>, // QUIC datagram接收计数 / QUIC dgram recv count
    pub blocked_uids: Arc<dashmap::DashSet<String>>,          // 封禁UID集合 / Blocked UIDs
    pub uid_rate_limits: Arc<dashmap::DashMap<String, (usize, usize, i64)>>, // UID限流 (limit, count, window_start_ms)
}

impl VConnectIMServer {
    /// 构建默认服务器实例 / Build default server instance
    pub fn new() -> Self {
        let directory = Arc::new(cluster::directory::Directory::new());
        let raft = Arc::new(cluster::raft::RaftCluster::new(
            directory.clone(),
            "node-local".to_string(),
        ));
        let plugin_registry = Arc::new(PluginRegistry::new());
        let remote_plugins = Arc::new(RemotePluginManager::new());
        let auth_plugin: Arc<dyn AuthPlugin> = Arc::new(DefaultAuthPlugin::new());
        Self {
            connections: Arc::new(DashMap::new()),
            webhook_config: None,
            auth_config: None,
            auth_plugin,
            plugin_registry,
            remote_plugins,
            plugin_config: Arc::new(RwLock::new(Value::Null)),
            acked_ids: Arc::new(DashMap::new()),
            node_id: "node-local".to_string(),
            directory,
            broker: cluster::broker::ShardBroker::new(),
            storage: storage::Storage::open_temporary().expect("open storage"),
            raft,
            rooms: Arc::new(DashMap::new()),
            uid_clients: Arc::new(DashMap::new()),
            quic_conn_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            quic_path_updates: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            quic_stream_sent: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            quic_dgram_sent: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            quic_stream_recv: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            quic_dgram_recv: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            blocked_uids: Arc::new(dashmap::DashSet::new()),
            uid_rate_limits: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// 配置Webhook / Configure webhook
    pub fn with_webhook_config(mut self, config: crate::config::WebhookConfigLite) -> Self {
        self.webhook_config = Some(config);
        self
    }

    /// 配置鉴权 / Configure auth
    pub fn with_auth_config(mut self, config: crate::config::AuthConfigLite) -> Self {
        self.auth_config = Some(config);
        self
    }

    /// 配置授权插件 / Configure custom auth plugin
    pub fn with_auth_plugin(mut self, plugin: Arc<dyn AuthPlugin>) -> Self {
        self.auth_plugin = plugin;
        self
    }

    /// 注册通用插件 / Register generic plugin
    pub fn with_plugin(self, plugin: Arc<dyn Plugin>) -> Self {
        self.plugin_registry.register(plugin);
        self
    }

    /// 配置节点并初始化存储 / Configure node and init storage
    pub fn with_node(
        mut self,
        node_id: String,
        directory: Arc<cluster::directory::Directory>,
    ) -> Self {
        self.node_id = node_id.clone();
        self.directory = directory.clone();
        self.directory.register_node(cluster::router::NodeInfo {
            node_id,
            weight: 1,
            is_alive: true,
        });
        let path = match v::get_global_config_manager() {
            Ok(cm) => cm.get_or(
                "storage.path",
                format!(
                    "{}/data/v-connect-im-{}",
                    env!("CARGO_MANIFEST_DIR"),
                    self.node_id
                ),
            ),
            Err(_) => format!(
                "{}/data/v-connect-im-{}",
                env!("CARGO_MANIFEST_DIR"),
                self.node_id
            ),
        };
        self.storage = storage::Storage::open(&path).expect("open storage");
        self
    }

    /// 配置Raft集群 / Configure raft cluster
    pub fn with_raft(mut self, raft: Arc<cluster::raft::RaftCluster>) -> Self {
        self.raft = raft;
        self
    }
}

/// 便捷克隆 / Convenience clone
impl Clone for VConnectIMServer {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            webhook_config: self.webhook_config.clone(),
            auth_config: self.auth_config.clone(),
            auth_plugin: self.auth_plugin.clone(),
            plugin_registry: self.plugin_registry.clone(),
            remote_plugins: self.remote_plugins.clone(),
            plugin_config: self.plugin_config.clone(),
            acked_ids: self.acked_ids.clone(),
            node_id: self.node_id.clone(),
            directory: self.directory.clone(),
            broker: cluster::broker::ShardBroker::new(),
            storage: storage::Storage::open_temporary().expect("open storage"),
            raft: self.raft.clone(),
            rooms: self.rooms.clone(),
            uid_clients: self.uid_clients.clone(),
            quic_conn_count: self.quic_conn_count.clone(),
            quic_path_updates: self.quic_path_updates.clone(),
            quic_stream_sent: self.quic_stream_sent.clone(),
            quic_dgram_sent: self.quic_dgram_sent.clone(),
            quic_stream_recv: self.quic_stream_recv.clone(),
            quic_dgram_recv: self.quic_dgram_recv.clone(),
            blocked_uids: self.blocked_uids.clone(),
            uid_rate_limits: self.uid_rate_limits.clone(),
        }
    }
}

impl VConnectIMServer {
    pub fn set_plugin_config(&self, value: Value) {
        *self.plugin_config.write() = value;
    }

    pub fn get_plugin_config(&self) -> Value {
        self.plugin_config.read().clone()
    }
}
