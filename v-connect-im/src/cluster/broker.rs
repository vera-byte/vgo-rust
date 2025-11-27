use dashmap::DashSet;

/// 分片代理 / Shard broker
#[derive(Clone)]
pub struct ShardBroker {
    #[allow(dead_code)]
    recent_ids: DashSet<String>, // 近期ID集合（当前未读取）/ Recent IDs set (currently not read)
}

impl ShardBroker {
    pub fn new() -> Self { Self { recent_ids: DashSet::new() } }
    #[allow(dead_code)]
    pub fn should_enqueue(&self, id: &str) -> bool {
        if self.recent_ids.contains(id) { return false; }
        self.recent_ids.insert(id.to_string());
        true
    }
}
