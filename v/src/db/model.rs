use crate::db::connection::get_pool;
use crate::db::error::Result;
use sqlx::Pool;
use sqlx::Postgres;

/// 通用模型元信息 Trait（表名与分库组）
/// Generic model meta trait (table name and group)
pub trait DbModel {
    fn table_name() -> &'static str;
    fn table_group() -> &'static str;
}

/// 为模型类型获取对应分组的连接池 / Get pool for model's group
pub async fn pool_for<M: DbModel>() -> Result<Pool<Postgres>> {
    get_pool(M::table_group()).await
}

/// 便捷宏：为模型实现 DbModel（使用模块内约定常量）
/// Helper macro: implement DbModel using module-level consts
#[macro_export]
macro_rules! impl_table_meta {
    ($ty:path, $table:path, $group:path) => {
        impl $crate::db::model::DbModel for $ty {
            fn table_name() -> &'static str {
                $table
            }
            fn table_group() -> &'static str {
                $group
            }
        }
    };
}

/// 列类型声明（用于插入/更新的序列化）
/// Column type declaration (for insert/update serialization)
#[derive(Clone, Copy)]
pub enum ColType {
    Text,
    Int64,
    Int16,
    Bool,
    Timestamp,
    Json,
    ArrayText,
}

/// 列定义 / Column definition
pub struct ColumnDef {
    pub name: &'static str,
    pub ty: ColType,
}

/// 可选的模型列规范 Trait（用于自动映射）
/// Optional model column spec trait (for auto mapping)
pub trait ModelSpec: DbModel {
    fn columns() -> &'static [ColumnDef];
}
