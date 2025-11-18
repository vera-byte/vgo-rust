use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use v::db::database::{DatabaseManager, DbPool};
const SQL_LIST: &str = "SELECT id, tenant_id, \
             (created_at AT TIME ZONE 'UTC') as created_at, \
             (updated_at AT TIME ZONE 'UTC') as updated_at, \
             username, email, mobile, password_hash, real_name, nick_name, avatar_url, \
             gender, birthday, id_card, country, province, city, district, address, postal_code, \
             status, is_email_verified, is_mobile_verified, \
             (last_login_time AT TIME ZONE 'UTC') as last_login_time, \
             last_login_ip::text as last_login_ip, \
             login_count, source, invite_code, referrer_id, remark, \
             (deleted_at AT TIME ZONE 'UTC') as deleted_at \
             FROM \"public\".\"user\" WHERE deleted_at IS NULL ORDER BY id ASC";
const SQL_INFO: &str = "SELECT id, tenant_id, \
             (created_at AT TIME ZONE 'UTC') as created_at, \
             (updated_at AT TIME ZONE 'UTC') as updated_at, \
             username, email, mobile, password_hash, real_name, nick_name, avatar_url, \
             gender, birthday, id_card, country, province, city, district, address, postal_code, \
             status, is_email_verified, is_mobile_verified, \
             (last_login_time AT TIME ZONE 'UTC') as last_login_time, \
             last_login_ip::text as last_login_ip, \
             login_count, source, invite_code, referrer_id, remark, \
             (deleted_at AT TIME ZONE 'UTC') as deleted_at \
             FROM \"public\".\"user\" WHERE id = $1 LIMIT 1";

pub async fn list() -> Result<Vec<crate::model::user::User>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::user::User>().await?;
    let DbPool::Postgres(p) = pool;
    let p_clone = p.clone();
    let rows: Vec<crate::model::user::User> =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let mut conn = p_clone
                .get_timeout(Duration::from_secs(2))
                .map_err(|e| anyhow!(e))?;
            let r: Vec<crate::model::user::User> = diesel::sql_query(SQL_LIST)
                .load(&mut conn)
                .map_err(|e| anyhow!(e))?;
            Ok(r)
        })
        .await??;
    Ok(rows)
}

pub async fn info(id: i64) -> Result<Option<crate::model::user::User>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::user::User>().await?;
    let DbPool::Postgres(p) = pool;
    let p_clone = p.clone();
    let rows: Vec<crate::model::user::User> =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let mut conn = p_clone
                .get_timeout(Duration::from_secs(2))
                .map_err(|e| anyhow!(e))?;
            let r: Vec<crate::model::user::User> = diesel::sql_query(SQL_INFO)
                .bind::<BigInt, _>(id)
                .load(&mut conn)
                .map_err(|e| anyhow!(e))?;
            Ok(r)
        })
        .await??;
    Ok(rows.into_iter().next())
}
use std::time::Duration;
