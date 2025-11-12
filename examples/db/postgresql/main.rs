use diesel::pg::PgConnection;
use diesel::prelude::*;
use v::db::database::{DatabaseManager, DbPool};
use tokio::time::{sleep, Duration};
mod model;
use model::{BaseSysConf, NewBaseSysConf};

diesel::table! {
    base_sys_conf (id) {
        id -> BigInt,
        tenant_id -> BigInt,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
        c_key -> Text,
        c_value -> Text,
    }
}

use diesel::dsl::*;

#[allow(dead_code)]
fn now_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

#[tokio::main]
async fn main() {
    // 通过配置文件 config/*.toml 读取 default 分组的数据库配置，并带重试与退避
    let pool = match init_pg_pool_with_retry("default", 5, Duration::from_millis(400)).await {
        Some(p) => p,
        None => {
            eprintln!(
                "无法初始化 PostgreSQL 连接池，请检查本地数据库是否可用或降低 [database.default.maxOpen]。"
            );
            return;
        }
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法从连接池获取 PostgreSQL 连接: {}", e);
            return;
        }
    };

    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS base_sys_conf (
            id BIGSERIAL PRIMARY KEY,
            tenant_id BIGINT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMP NULL,
            c_key TEXT NOT NULL,
            c_value TEXT NOT NULL
        )",
    )
    .execute(&mut conn)
    .expect("创建表失败");

    println!(
        "模型标识: table={}, group={}",
        <BaseSysConf as v::db::database::Model>::table_name(),
        <BaseSysConf as v::db::database::Model>::group_name()
    );

    let rows = vec![
        NewBaseSysConf { tenantId: 1, cKey: "site_name".into(), cValue: "vgo".into() },
        NewBaseSysConf { tenantId: 1, cKey: "theme".into(), cValue: "dark".into() },
    ];
    diesel::insert_into(base_sys_conf::table)
        .values(&rows)
        .execute(&mut conn)
        .expect("插入失败");

    let list: Vec<BaseSysConf> = base_sys_conf::table
        .select(BaseSysConf::as_select())
        .order(base_sys_conf::id.asc())
        .load(&mut conn)
        .expect("查询失败");

    println!("查询结果: {:?}", list);
}

/// 带重试与线性退避的 PostgreSQL 连接池初始化
async fn init_pg_pool_with_retry(
    group: &str,
    max_attempts: u32,
    base_delay: Duration,
) -> Option<diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>> {
    for attempt in 1..=max_attempts {
        match DatabaseManager::get_any_pool_by_group(group).await {
            Ok(DbPool::Postgres(p)) => return Some(p),
            Ok(DbPool::Sqlite(_)) => {
                eprintln!("[db] 组 '{}' 当前配置为 sqlite，请改为 postgresql", group);
                return None;
            }
            #[cfg(feature = "mysql_backend")]
            Ok(DbPool::Mysql(_)) => {
                eprintln!("[db] 组 '{}' 当前配置为 mysql，请改为 postgresql", group);
                return None;
            }
            Err(e) => {
                eprintln!(
                    "[db] 初始化连接池失败(第{}/{}次): {}",
                    attempt, max_attempts, e
                );
                let delay = base_delay * attempt;
                sleep(delay).await;
            }
        }
    }
    None
}
