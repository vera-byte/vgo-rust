use serde::Serialize;

pub fn ok_body<T: Serialize>(data: &T) -> serde_json::Value {
    serde_json::to_value(data).unwrap_or(serde_json::Value::Null)
}

pub fn build_etag(content: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(content);
    let hash = hasher.finalize();
    format!("\"{:x}\"", hash)
}
