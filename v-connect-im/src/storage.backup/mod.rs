use anyhow::Result;
use sled::{Db, Tree}; // 移除未使用的IVec导入 / Remove unused IVec import
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

#[derive(Clone, Debug)]
pub struct Storage {
    #[allow(dead_code)]
    db: Db, // 目前未直接读取db句柄 / DB handle currently not read directly
    wal: Tree,
    offline: Tree,
    room_members: Tree, // 房间成员映射 / Room members mapping
    reads: Tree,        // 已读映射 / Read receipts
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub from_client_id: String,
    pub to_client_id: String,
    pub content: serde_json::Value,
    pub timestamp: i64,
    pub msg_type: String,
    pub room_id: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OfflineRecord {
    pub message_id: String,
    pub from_uid: Option<String>,
    pub to_uid: String,
    pub room_id: Option<String>,
    pub content: serde_json::Value,
    pub timestamp: i64,
    pub msg_type: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReadReceipt {
    pub message_id: String,
    pub uid: String,
    pub timestamp: i64,
}

impl Storage {
    pub fn open(path: &str) -> Result<Self> {
        let db = sled::open(path)?;
        let wal = db.open_tree("wal")?;
        let offline = db.open_tree("offline")?;
        let room_members = db.open_tree("room_members")?;
        let reads = db.open_tree("reads")?;
        Ok(Self {
            db,
            wal,
            offline,
            room_members,
            reads,
        })
    }

    pub fn open_temporary() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
        let wal = db.open_tree("wal")?;
        let offline = db.open_tree("offline")?;
        let room_members = db.open_tree("room_members")?;
        let reads = db.open_tree("reads")?;
        Ok(Self {
            db,
            wal,
            offline,
            room_members,
            reads,
        })
    }

    pub fn append(&self, rec: &MessageRecord) -> Result<()> {
        let key = format!("{}:{}", rec.timestamp, rec.message_id);
        let val = serde_json::to_vec(rec)?;
        self.wal.insert(key.as_bytes(), val)?;
        self.wal.flush()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, message_id: &str) -> Result<Option<MessageRecord>> {
        let prefix = format!(":{}", message_id);
        for item in self.wal.iter() {
            let (k, v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if ks.ends_with(&prefix) {
                let rec: MessageRecord = serde_json::from_slice(&v)?;
                return Ok(Some(rec));
            }
        }
        Ok(None)
    }

    #[allow(dead_code)]
    pub fn clear_wal(&self) -> Result<()> {
        let keys: Vec<Vec<u8>> = self
            .wal
            .iter()
            .filter_map(|item| item.ok().map(|(k, _)| k.to_vec()))
            .collect();
        for k in keys {
            self.wal.remove(k)?;
        }
        self.wal.flush()?;
        Ok(())
    }

    pub fn store_offline(&self, rec: &OfflineRecord) -> Result<()> {
        let key = format!("{}:{}:{}", rec.to_uid, rec.timestamp, rec.message_id);
        let val = serde_json::to_vec(rec)?;
        self.offline.insert(key.as_bytes(), val)?;
        self.offline.flush()?;
        Ok(())
    }

    pub fn pull_offline(&self, to_uid: &str, limit: usize) -> Result<Vec<OfflineRecord>> {
        let prefix = format!("{}:", to_uid);
        let mut res = Vec::new();
        for item in self.offline.scan_prefix(prefix.as_bytes()) {
            let (_k, v) = item?;
            let rec: OfflineRecord = serde_json::from_slice(&v)?;
            res.push(rec);
            if res.len() >= limit {
                break;
            }
        }
        Ok(res)
    }

    /// 分页拉取离线消息 / Paginated offline pull
    #[allow(dead_code)]
    pub fn pull_offline_paginated(
        &self,
        to_uid: &str,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<(Vec<OfflineRecord>, Option<String>)> {
        // cursor 设计为完整键 `uid:timestamp:message_id` / cursor is full key `uid:timestamp:message_id`
        let start_key = cursor.unwrap_or_else(|| format!("{}:", to_uid));
        let mut items = Vec::new();
        let mut next_cursor: Option<String> = None;
        for item in self.offline.range(start_key.as_bytes()..) {
            let (k, v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if !ks.starts_with(&format!("{}:", to_uid)) {
                break;
            }
            // 跳过等于起始游标的第一项 / skip the first item equal to start cursor
            if ks == start_key {
                continue;
            }
            let rec: OfflineRecord = serde_json::from_slice(&v)?;
            items.push(rec);
            next_cursor = Some(ks);
            if items.len() >= limit {
                break;
            }
        }
        Ok((items, next_cursor))
    }

    /// 分页拉取（带时间过滤）/ Paginated offline pull with time filter
    #[allow(dead_code)]
    pub fn pull_offline_by_time(
        &self,
        to_uid: &str,
        cursor: Option<String>,
        limit: usize,
        since_ts: Option<i64>,
        until_ts: Option<i64>,
    ) -> Result<(Vec<OfflineRecord>, Option<String>)> {
        let start_key = cursor.unwrap_or_else(|| format!("{}:", to_uid));
        let mut items = Vec::new();
        let mut next_cursor: Option<String> = None;
        for item in self.offline.range(start_key.as_bytes()..) {
            let (k, v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if !ks.starts_with(&format!("{}:", to_uid)) {
                break;
            }
            if ks == start_key {
                continue;
            }
            // 解析时间戳 / parse timestamp
            let parts: Vec<&str> = ks.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let ts = parts[1].parse::<i64>().unwrap_or_default();
            if let Some(since) = since_ts {
                if ts < since {
                    continue;
                }
            }
            if let Some(until) = until_ts {
                if ts > until {
                    continue;
                }
            }
            let rec: OfflineRecord = serde_json::from_slice(&v)?;
            items.push(rec);
            next_cursor = Some(ks);
            if items.len() >= limit {
                break;
            }
        }
        Ok((items, next_cursor))
    }

    pub fn ack_offline(&self, to_uid: &str, message_ids: &[String]) -> Result<usize> {
        let mut removed = 0usize;
        for item in self.offline.iter() {
            let (k, v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if ks.starts_with(&format!("{}:", to_uid)) {
                let rec: OfflineRecord = serde_json::from_slice(&v)?;
                if message_ids.iter().any(|id| id == &rec.message_id) {
                    self.offline.remove(k)?;
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            self.offline.flush()?;
        }
        Ok(removed)
    }

    /// 清理指定uid在某时间之前的离线消息（限制数量） / Cleanup offline messages before timestamp
    pub fn cleanup_offline(&self, to_uid: &str, before_ts: i64, limit: usize) -> Result<usize> {
        let prefix = format!("{}:", to_uid);
        let mut removed = 0usize;
        for item in self.offline.scan_prefix(prefix.as_bytes()) {
            let (k, _v) = item?; // 未使用的值变量改为下划线前缀 / Prefix unused value variable with underscore
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            let parts: Vec<&str> = ks.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let ts = parts[1].parse::<i64>().unwrap_or_default();
            if ts <= before_ts {
                self.offline.remove(k)?;
                removed += 1;
                if removed >= limit {
                    break;
                }
            }
        }
        if removed > 0 {
            self.offline.flush()?;
        }
        Ok(removed)
    }

    /// 房间成员持久化：添加成员 / Persist room member: add
    pub fn add_room_member(&self, room_id: &str, uid: &str) -> Result<()> {
        let key = format!("{}:{}", room_id, uid);
        self.room_members.insert(key.as_bytes(), b"1")?;
        self.room_members.flush()?;
        Ok(())
    }

    /// 房间成员持久化：移除成员 / Persist room member: remove
    pub fn remove_room_member(&self, room_id: &str, uid: &str) -> Result<()> {
        let key = format!("{}:{}", room_id, uid);
        self.room_members.remove(key.as_bytes())?;
        self.room_members.flush()?;
        Ok(())
    }

    /// 房间成员持久化：列出成员 / Persist room member: list
    pub fn list_room_members(&self, room_id: &str) -> Result<Vec<String>> {
        let prefix = format!("{}:", room_id);
        let mut res = Vec::new();
        for item in self.room_members.scan_prefix(prefix.as_bytes()) {
            let (k, _v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if let Some((_rid, uid)) = ks.split_once(':') {
                res.push(uid.to_string());
            }
        }
        Ok(res)
    }

    /// 统计某UID的离线消息数量 / Count offline messages for uid
    pub fn offline_count(&self, to_uid: &str) -> Result<usize> {
        let prefix = format!("{}:", to_uid);
        let mut count = 0usize;
        for item in self.offline.scan_prefix(prefix.as_bytes()) {
            let _ = item?; // iterate
            count += 1;
        }
        Ok(count)
    }

    /// 统计某UID在房间的未读条数 / Count offline messages for uid and room
    pub fn offline_count_by_room(&self, to_uid: &str, room_id: &str) -> Result<usize> {
        let prefix = format!("{}:", to_uid);
        let mut count = 0usize;
        for item in self.offline.scan_prefix(prefix.as_bytes()) {
            let (_k, v) = item?;
            let rec: OfflineRecord = serde_json::from_slice(&v)?;
            if rec.room_id.as_deref() == Some(room_id) {
                count += 1;
            }
        }
        Ok(count)
    }

    /// 强制离线配额：删除最早的若干条 / Enforce offline quota: remove oldest
    pub fn enforce_offline_quota(
        &self,
        to_uid: &str,
        max_count: usize,
        cleanup_batch: usize,
    ) -> Result<usize> {
        let current = self.offline_count(to_uid)?;
        if current < max_count {
            return Ok(0);
        }
        let need = std::cmp::min(cleanup_batch, current.saturating_sub(max_count) + 1);
        let prefix = format!("{}:", to_uid);
        let mut removed = 0usize;
        for item in self.offline.scan_prefix(prefix.as_bytes()) {
            let (k, _v) = item?;
            self.offline.remove(k)?;
            removed += 1;
            if removed >= need {
                break;
            }
        }
        if removed > 0 {
            self.offline.flush()?;
        }
        Ok(removed)
    }

    pub fn record_read(&self, rr: &ReadReceipt) -> Result<()> {
        let key = format!("{}:{}", rr.uid, rr.message_id);
        let val = serde_json::to_vec(rr)?;
        self.reads.insert(key.as_bytes(), val)?;
        self.reads.flush()?;
        Ok(())
    }

    pub fn list_reads(&self, uid: &str, limit: usize) -> Result<Vec<ReadReceipt>> {
        let prefix = format!("{}:", uid);
        let mut res = Vec::new();
        for item in self.reads.scan_prefix(prefix.as_bytes()) {
            let (_k, v) = item?;
            let rr: ReadReceipt = serde_json::from_slice(&v)?;
            res.push(rr);
            if res.len() >= limit {
                break;
            }
        }
        Ok(res)
    }

    /// 列出所有房间ID / List all room ids
    pub fn list_rooms(&self) -> Result<Vec<String>> {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for item in self.room_members.iter() {
            let (k, _v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if let Some((rid, _uid)) = ks.split_once(':') {
                set.insert(rid.to_string());
            }
        }
        Ok(set.into_iter().collect())
    }

    /// 房间列表（前缀与限制）/ List rooms by prefix and limit
    pub fn list_rooms_by_prefix(
        &self,
        prefix: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<String>> {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        if let Some(p) = prefix {
            for item in self.room_members.scan_prefix(p.as_bytes()) {
                let (k, _v) = item?;
                let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
                if let Some((rid, _uid)) = ks.split_once(':') {
                    if rid.starts_with(p) {
                        set.insert(rid.to_string());
                    }
                    if let Some(lim) = limit {
                        if set.len() >= lim {
                            break;
                        }
                    }
                }
            }
        } else {
            for item in self.room_members.iter() {
                let (k, _v) = item?;
                let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
                if let Some((rid, _uid)) = ks.split_once(':') {
                    set.insert(rid.to_string());
                    if let Some(lim) = limit {
                        if set.len() >= lim {
                            break;
                        }
                    }
                }
            }
        }
        Ok(set.into_iter().collect())
    }

    /// 房间成员分页与前缀过滤 / Paginated room members with prefix filter
    pub fn list_room_members_paginated(
        &self,
        room_id: &str,
        uid_prefix: Option<&str>,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<(Vec<String>, Option<String>)> {
        let prefix = format!("{}:", room_id);
        let mut res: Vec<String> = Vec::new();
        let mut next_cursor: Option<String> = None;
        for item in self.room_members.scan_prefix(prefix.as_bytes()) {
            let (k, _v) = item?;
            let ks = String::from_utf8(k.to_vec()).unwrap_or_default();
            if let Some(cur) = &cursor {
                if &ks <= cur {
                    continue;
                }
            }
            if let Some((_rid, uid)) = ks.split_once(':') {
                if let Some(pfx) = uid_prefix {
                    if !uid.starts_with(pfx) {
                        continue;
                    }
                }
                res.push(uid.to_string());
                next_cursor = Some(ks);
                if res.len() >= limit {
                    break;
                }
            }
        }
        Ok((res, next_cursor))
    }

    #[allow(dead_code)]
    pub fn create_snapshot(&self, path: &str) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        for item in self.wal.iter() {
            let (_k, v) = item?;
            writer.write_all(&v)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn restore_from_snapshot(&self, path: &str) -> Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let rec: MessageRecord = serde_json::from_str(&line)?;
            self.append(&rec)?;
        }
        Ok(())
    }
}

impl Storage {
    /// 按用户查询历史消息（按时间过滤与限制）/ List message history by user with time filters
    pub fn list_messages_by_user(
        &self,
        user: &str,
        peer: Option<&str>,
        since_ts: Option<i64>,
        until_ts: Option<i64>,
        limit: usize,
    ) -> Result<Vec<MessageRecord>> {
        let mut res: Vec<MessageRecord> = Vec::new();
        for item in self.wal.iter() {
            let (_k, v) = item?;
            let rec: MessageRecord = serde_json::from_slice(&v)?;
            let involved = rec.to_client_id == user || rec.from_client_id == user;
            if !involved {
                continue;
            }
            if let Some(p) = peer {
                let with_peer = rec.to_client_id == p || rec.from_client_id == p;
                if !with_peer {
                    continue;
                }
            }
            if let Some(since) = since_ts {
                if rec.timestamp < since {
                    continue;
                }
            }
            if let Some(until) = until_ts {
                if rec.timestamp > until {
                    continue;
                }
            }
            res.push(rec);
            if res.len() >= limit {
                break;
            }
        }
        Ok(res)
    }
}
