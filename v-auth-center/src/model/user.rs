// 用户模型与 Diesel 映射 / User model and Diesel mapping
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// 表模块：users（包含公共审计列与业务列） / Table module: users (common audit columns + business columns)
diesel::table! {
    users (id) {
        id -> BigInt,
        tenant_id -> BigInt,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        username -> Text,
        email -> Text,
        password_hash -> Text,
    }
}

// 业务模型：统一公共字段为 camelCase，并映射到 snake_case 列
// Business model: unify common fields in camelCase, map to snake_case columns
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, AsChangeset, Insertable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i64,
    #[diesel(column_name = tenant_id)]
    pub tenantId: i64,
    #[diesel(column_name = created_at)]
    pub createTime: chrono::NaiveDateTime,
    #[diesel(column_name = updated_at)]
    pub updateTime: chrono::NaiveDateTime,
    pub username: String,
    pub email: String,
    #[diesel(column_name = password_hash)]
    pub passwordHash: String,
}

// 指定表名与数据库组名 / Bind table and database group
impl v::db::database::Model for User {
    fn table_name() -> &'static str {
        "users"
    }
    fn group_name() -> &'static str {
        "default"
    }
}
