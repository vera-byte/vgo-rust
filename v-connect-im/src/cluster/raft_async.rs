use anyhow::Result;
use dashmap::DashMap;
use std::sync::{Arc, RwLock};

use crate::storage::{MessageRecord, Storage};

/// 内存版Raft存储（模拟）/ In-memory Raft storage (mock)
#[derive(Clone)]
pub struct MockRaftStorage {
    pub storage: Storage,
    pub last_index: Arc<RwLock<u64>>, // 日志索引 / log index
}

impl MockRaftStorage {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            last_index: Arc::new(RwLock::new(0)),
        }
    }
    pub fn append(&self, rec: &MessageRecord) -> Result<u64> {
        self.storage.append(rec)?;
        let mut idx = self.last_index.write().unwrap();
        *idx += 1;
        Ok(*idx)
    }
}

/// 内存版Raft网络（模拟）/ In-memory Raft network (mock)
#[derive(Clone)]
pub struct MockRaftNetwork {
    pub peers: Arc<DashMap<String, MockRaftStorage>>, // 节点存储 / node storage
    pub leader_id: Arc<RwLock<String>>,               // 当前Leader / current leader
}

impl MockRaftNetwork {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(DashMap::new()),
            leader_id: Arc::new(RwLock::new(String::new())),
        }
    }
    pub fn register(&self, node_id: &str, storage: MockRaftStorage) {
        self.peers.insert(node_id.to_string(), storage);
    }
    pub fn elect(&self, node_id: &str) {
        *self.leader_id.write().unwrap() = node_id.to_string();
    }
    pub fn leader(&self) -> String {
        self.leader_id.read().unwrap().clone()
    }

    pub async fn client_write(&self, from_node: &str, rec: &MessageRecord) -> Result<()> {
        if from_node != self.leader() {
            return Err(anyhow::anyhow!("not leader"));
        }
        // 复制到多数派 / replicate to majority
        let total = self.peers.len().max(1);
        let quorum = (total / 2) + 1;
        let mut acks = 0u64;
        for entry in self.peers.iter() {
            let s = entry.value();
            let _ = s.append(rec)?;
            acks += 1;
        }
        if acks >= quorum as u64 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("replication quorum not met"))
        }
    }
}

/// 集群包装（模拟）/ Cluster wrapper (mock)
#[derive(Clone)]
pub struct AsyncRaftCluster {
    pub net: MockRaftNetwork,
    pub nodes: Arc<DashMap<String, MockRaftStorage>>, // 节点集合 / nodes
}

impl AsyncRaftCluster {
    pub fn new() -> Self {
        Self {
            net: MockRaftNetwork::new(),
            nodes: Arc::new(DashMap::new()),
        }
    }
    pub fn add_node(&self, node_id: &str, storage: Storage) {
        let s = MockRaftStorage::new(storage);
        self.nodes.insert(node_id.to_string(), s.clone());
        self.net.register(node_id, s);
    }
    pub fn elect(&self, node_id: &str) {
        self.net.elect(node_id);
    }
    pub async fn write(&self, node_id: &str, rec: &MessageRecord) -> Result<()> {
        self.net.client_write(node_id, rec).await
    }

    pub fn install_snapshot_from_leader(&self, path_prefix: &str) -> Result<()> {
        let leader = self.net.leader();
        let leader_store = self
            .nodes
            .get(&leader)
            .ok_or_else(|| anyhow::anyhow!("leader not found"))?
            .value()
            .clone();
        let snap_path = format!("{}-{}.snap", path_prefix, leader);
        leader_store.storage.create_snapshot(&snap_path)?;
        for entry in self.nodes.iter() {
            let id = entry.key();
            if *id == leader {
                continue;
            }
            let s = entry.value();
            s.storage.clear_wal()?;
            s.storage.restore_from_snapshot(&snap_path)?;
        }
        Ok(())
    }
}
