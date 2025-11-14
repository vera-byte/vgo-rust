use anyhow::Result;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Date, Integer, SmallInt, Text, Timestamp};
use v::db::database::{DatabaseManager, DbPool};

#[derive(QueryableByName)]
struct UserRow {
    #[diesel(sql_type = BigInt, column_name = id)]
    id: i64,
    #[diesel(sql_type = diesel::sql_types::Nullable<Integer>, column_name = tenant_id)]
    tenant_id: Option<i32>,
    #[diesel(sql_type = Timestamp, column_name = create_time)]
    create_time: chrono::NaiveDateTime,
    #[diesel(sql_type = Timestamp, column_name = update_time)]
    update_time: chrono::NaiveDateTime,
    #[diesel(sql_type = Text, column_name = username)]
    username: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = email)]
    email: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = mobile)]
    mobile: Option<String>,
    #[diesel(sql_type = Text, column_name = password_hash)]
    password_hash: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = real_name)]
    real_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = nick_name)]
    nick_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = avatar_url)]
    avatar_url: Option<String>,
    #[diesel(sql_type = SmallInt, column_name = gender)]
    gender: i16,
    #[diesel(sql_type = diesel::sql_types::Nullable<Date>, column_name = birthday)]
    birthday: Option<chrono::NaiveDate>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = id_card)]
    id_card: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = country)]
    country: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = province)]
    province: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = city)]
    city: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = district)]
    district: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = address)]
    address: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = postal_code)]
    postal_code: Option<String>,
    #[diesel(sql_type = SmallInt, column_name = status)]
    status: i16,
    #[diesel(sql_type = Bool, column_name = is_email_verified)]
    is_email_verified: bool,
    #[diesel(sql_type = Bool, column_name = is_mobile_verified)]
    is_mobile_verified: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<Timestamp>, column_name = last_login_time)]
    last_login_time: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = last_login_ip)]
    last_login_ip: Option<String>,
    #[diesel(sql_type = Integer, column_name = login_count)]
    login_count: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = source)]
    source: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = invite_code)]
    invite_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>, column_name = referrer_id)]
    referrer_id: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>, column_name = remark)]
    remark: Option<String>,
}

pub async fn list() -> Result<Vec<crate::model::user::User>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::user::User>().await?;
    if let DbPool::Postgres(p) = pool {
        let p_clone = p.clone();
        let rows: Vec<crate::model::user::User> =
            tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<crate::model::user::User>> {
                let mut conn = p_clone
                    .get_timeout(Duration::from_secs(2))
                    .map_err(|e| anyhow::anyhow!(e))?;
                let r: Vec<crate::model::user::User> = diesel::sql_query(
                    "SELECT id, tenant_id, \
                     (create_time AT TIME ZONE 'UTC') as create_time, \
                     (update_time AT TIME ZONE 'UTC') as update_time, \
                     username, email, mobile, password_hash, real_name, nick_name, avatar_url, \
                     gender, birthday, id_card, country, province, city, district, address, postal_code, \
                     status, is_email_verified, is_mobile_verified, \
                     (last_login_time AT TIME ZONE 'UTC') as last_login_time, \
                     last_login_ip::text as last_login_ip, \
                     login_count, source, invite_code, referrer_id, remark \
                     FROM \"public\".\"user\" ORDER BY id ASC",
                )
                    .load(&mut conn)
                    .map_err(|e| anyhow::anyhow!(e))?;
                Ok::<Vec<crate::model::user::User>, anyhow::Error>(r)
            })
            .await??;
        return Ok(rows);
    }
    anyhow::bail!("only postgresql is supported")
}

pub async fn info(id: i64) -> Result<Option<crate::model::user::User>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::user::User>().await?;
    if let DbPool::Postgres(p) = pool {
        let p_clone = p.clone();
        let rows: Vec<crate::model::user::User> = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<crate::model::user::User>> {
            let mut conn = p_clone
                .get_timeout(Duration::from_secs(2))
                .map_err(|e| anyhow::anyhow!(e))?;
            let r: Vec<crate::model::user::User> = diesel::sql_query(
                "SELECT id, tenant_id, \
                 (create_time AT TIME ZONE 'UTC') as create_time, \
                 (update_time AT TIME ZONE 'UTC') as update_time, \
                 username, email, mobile, password_hash, real_name, nick_name, avatar_url, \
                 gender, birthday, id_card, country, province, city, district, address, postal_code, \
                 status, is_email_verified, is_mobile_verified, \
                 (last_login_time AT TIME ZONE 'UTC') as last_login_time, \
                 last_login_ip::text as last_login_ip, \
                 login_count, source, invite_code, referrer_id, remark \
                 FROM \"public\".\"user\" WHERE id = $1 LIMIT 1",
            )
            .bind::<BigInt, _>(id)
            .load(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
            Ok::<Vec<crate::model::user::User>, anyhow::Error>(r)
        })
        .await??;
        return Ok(rows.into_iter().next());
    }
    anyhow::bail!("only postgresql is supported")
}
use std::time::Duration;
