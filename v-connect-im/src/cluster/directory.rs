use dashmap::DashMap;
use std::sync::Arc;

use super::router::NodeInfo;
use crate::VConnectIMServer;

/// 目录服务 / Directory service
#[derive(Clone)]
pub struct Directory {
    pub nodes: Arc<DashMap<String, NodeInfo>>, // 节点元信息 / Node metadata
    pub clients: Arc<DashMap<String, String>>, // 客户端到节点映射 / Client -> node
    pub servers: Arc<DashMap<String, Arc<VConnectIMServer>>>, // 节点到服务器实例 / Node -> server
}

impl Directory {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            clients: Arc::new(DashMap::new()),
            servers: Arc::new(DashMap::new()),
        }
    }

    pub fn register_node(&self, info: NodeInfo) {
        self.nodes.insert(info.node_id.clone(), info);
    }

    #[allow(dead_code)]
    pub fn mark_alive(&self, node_id: &str, alive: bool) {
        if let Some(mut n) = self.nodes.get_mut(node_id) {
            n.is_alive = alive;
        }
    }

    pub fn register_server(&self, node_id: &str, server: Arc<VConnectIMServer>) {
        self.servers.insert(node_id.to_string(), server);
    }

    pub fn register_client_location(&self, client_id: &str, node_id: &str) {
        self.clients
            .insert(client_id.to_string(), node_id.to_string());
    }

    pub fn locate_client(&self, client_id: &str) -> Option<String> {
        self.clients.get(client_id).map(|v| v.clone())
    }

    pub fn list_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.iter().map(|it| it.value().clone()).collect()
    }

    pub fn get_server(&self, node_id: &str) -> Option<Arc<VConnectIMServer>> {
        self.servers.get(node_id).map(|s| s.value().clone())
    }
}
