use anyhow::Result;
use dashmap::DashMap;
use std::sync::{Arc, RwLock};

use super::directory::Directory;
use crate::storage::MessageRecord;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Leader,
    Follower,
} // 角色枚举（当前未使用）/ Role enum (currently unused)

#[derive(Clone)]
pub struct RaftCluster {
    directory: Arc<Directory>,
    leader_id: Arc<RwLock<String>>, // 当前Leader / Current leader
    commit_count: Arc<DashMap<String, u64>>, // 每节点提交计数 / Commit count per node
}

impl RaftCluster {
    pub fn new(directory: Arc<Directory>, leader_id: String) -> Self {
        Self {
            directory,
            leader_id: Arc::new(RwLock::new(leader_id)),
            commit_count: Arc::new(DashMap::new()),
        }
    }

    #[allow(dead_code)]
    pub fn set_leader(&self, leader_id: String) {
        if let Ok(mut l) = self.leader_id.write() {
            *l = leader_id;
        }
    }

    pub fn get_leader(&self) -> String {
        self.leader_id.read().map(|l| l.clone()).unwrap_or_default()
    }

    pub fn append_entry_as(&self, node_id: &str, rec: &MessageRecord) -> Result<()> {
        let leader = self.get_leader();
        if node_id != leader {
            return Err(anyhow::anyhow!("not leader"));
        }

        let nodes = self.directory.list_nodes();
        let total = nodes.len().max(1);
        let quorum = (total / 2) + 1;
        let mut acks = 0u64;
        // 本地先写入 / write local first
        if let Some(local) = self.directory.get_server(node_id) {
            local.storage.append(rec)?;
            acks += 1;
        } else {
            return Err(anyhow::anyhow!("server not registered"));
        }
        for n in nodes {
            if n.node_id == node_id {
                continue;
            }
            if let Some(server) = self.directory.get_server(&n.node_id) {
                if server.storage.append(rec).is_ok() {
                    acks += 1;
                }
            }
        }
        if acks >= quorum as u64 {
            let cnt = self.commit_count.get(node_id).map(|v| *v).unwrap_or(0);
            self.commit_count.insert(node_id.to_string(), cnt + 1);
            Ok(())
        } else {
            Err(anyhow::anyhow!("replication quorum not met"))
        }
    }
}
