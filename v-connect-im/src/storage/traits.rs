use anyhow::Result;
use super::{MessageRecord, OfflineRecord, ReadReceipt, Storage};

/// 存储扩展trait，便于测试替换 / Storage extension trait for testability
pub trait StorageExt {
    fn append(&self, rec: &MessageRecord) -> Result<()>;
    fn store_offline(&self, rec: &OfflineRecord) -> Result<()>;
    fn pull_offline(&self, to_uid: &str, limit: usize) -> Result<Vec<OfflineRecord>>;
    fn ack_offline(&self, to_uid: &str, ids: &[String]) -> Result<usize>;
    fn list_rooms(&self) -> Result<Vec<String>>;
    fn list_room_members(&self, room_id: &str) -> Result<Vec<String>>;
    fn enforce_offline_quota(&self, to_uid: &str, max_count: usize, cleanup_batch: usize) -> Result<usize>;
}

impl StorageExt for Storage {
    fn append(&self, rec: &MessageRecord) -> Result<()> { Storage::append(self, rec) }
    fn store_offline(&self, rec: &OfflineRecord) -> Result<()> { Storage::store_offline(self, rec) }
    fn pull_offline(&self, to_uid: &str, limit: usize) -> Result<Vec<OfflineRecord>> { Storage::pull_offline(self, to_uid, limit) }
    fn ack_offline(&self, to_uid: &str, ids: &[String]) -> Result<usize> { self.ack_offline(to_uid, ids) }
    fn list_rooms(&self) -> Result<Vec<String>> { Storage::list_rooms(self) }
    fn list_room_members(&self, room_id: &str) -> Result<Vec<String>> { Storage::list_room_members(self, room_id) }
    fn enforce_offline_quota(&self, to_uid: &str, max_count: usize, cleanup_batch: usize) -> Result<usize> { Storage::enforce_offline_quota(self, to_uid, max_count, cleanup_batch) }
}

