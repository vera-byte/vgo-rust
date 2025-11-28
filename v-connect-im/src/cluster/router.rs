use sha2::{Digest, Sha256};

/// 节点信息 / Node information
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub node_id: String,
    pub weight: u32,
    pub is_alive: bool,
}

/// HRW一致性哈希选择节点 / HRW consistent hashing node selection
#[allow(dead_code)]
pub fn hrw_select_node(key: &str, nodes: &[NodeInfo]) -> Option<NodeInfo> {
    if nodes.is_empty() {
        return None;
    }
    let mut best: Option<(u128, NodeInfo)> = None;
    for n in nodes.iter().filter(|n| n.is_alive) {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hasher.update(n.node_id.as_bytes());
        let hash = hasher.finalize();
        let score = u128::from_le_bytes(hash[0..16].try_into().unwrap()) * n.weight as u128;
        match &best {
            None => best = Some((score, n.clone())),
            Some((cur, _)) if score > *cur => best = Some((score, n.clone())),
            _ => {}
        }
    }
    best.map(|(_, n)| n)
}
