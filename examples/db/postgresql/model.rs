//! 示例模型定义：与 PostgreSQL 表结构完全对齐，业务模型直接用于 Queryable/Insertable。
//! 列使用：id、tenant_id、created_at、updated_at、deleted_at、c_key、c_value。

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
// 将表模块引入当前作用域，便于 #[diesel(table_name = ...)] 属性解析
use crate::base_sys_conf;

#[derive(Serialize, Deserialize, Debug, Clone, Queryable, Selectable, AsChangeset)]
#[diesel(table_name = base_sys_conf)]
pub struct BaseSysConf {
    pub id: i64,
    #[diesel(column_name = tenant_id)]
    pub tenantId: i64,
    #[diesel(column_name = created_at)]
    pub createdAt: chrono::NaiveDateTime,
    #[diesel(column_name = updated_at)]
    pub updatedAt: chrono::NaiveDateTime,
    #[diesel(column_name = deleted_at)]
    pub deletedAt: Option<chrono::NaiveDateTime>,
    #[diesel(column_name = c_key)]
    pub cKey: String,
    #[diesel(column_name = c_value)]
    pub cValue: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Insertable)]
#[diesel(table_name = base_sys_conf)]
pub struct NewBaseSysConf {
    #[diesel(column_name = tenant_id)]
    pub tenantId: i64,
    #[diesel(column_name = c_key)]
    pub cKey: String,
    #[diesel(column_name = c_value)]
    pub cValue: String,
}

impl v::db::database::Model for BaseSysConf {
    fn table_name() -> &'static str {
        "base_sys_conf"
    }
    fn group_name() -> &'static str {
        "default"
    }
}
