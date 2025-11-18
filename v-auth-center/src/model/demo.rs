use serde::{Deserialize, Serialize};
use v::db::model::{ColType, ColumnDef, ModelSpec};

// 示例模型 / Demo model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demo {
    pub id: i64,
    pub tenant_id: Option<i64>,
    pub name: String,
}
pub const TABLE_NAME: &str = "demo";
pub const TABLE_GROUP: &str = "default";

// 可选列映射（用于构建器插入/更新） / Optional column spec
impl ModelSpec for Demo {
    fn columns() -> &'static [ColumnDef] {
        static COLS: &[ColumnDef] = &[
            ColumnDef {
                name: "id",
                ty: ColType::Int64,
            },
            ColumnDef {
                name: "tenant_id",
                ty: ColType::Int64,
            },
            ColumnDef {
                name: "name",
                ty: ColType::Text,
            },
        ];
        COLS
    }
}
