use serde::{Deserialize, Serialize};
// 示例模型 / Demo models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demo2 {
    pub id: i64,
    pub tenant_id: Option<i64>,
    pub name: String,
    pub value: String,
}
pub const TABLE_NAME: &str = "demo2";
pub const TABLE_GROUP: &str = "default";
