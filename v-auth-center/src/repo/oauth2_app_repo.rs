use anyhow::{anyhow, Result};
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use std::time::Duration;
use tracing::info;

fn expand_sql_with_binds(sql: &str, binds: &[String]) -> String {
    let mut s = sql.to_string();
    for (i, b) in binds.iter().enumerate() {
        let ph = format!("${}", i + 1);
        s = s.replace(&ph, b);
    }
    s
}
use v::db::database::{DatabaseManager, DbPool};

#[derive(QueryableByName)]
struct AppJsonRow {
    #[diesel(sql_type = Text, column_name = data)]
    data: String,
}

const SQL_LIST: &str = "SELECT row_to_json(t.*)::text AS data FROM (SELECT * FROM \"public\".\"oauth2_app\" WHERE deleted_at IS NULL ORDER BY id ASC) t";
const SQL_INFO: &str = "SELECT row_to_json(t.*)::text AS data FROM (SELECT * FROM \"public\".\"oauth2_app\" WHERE id = $1 LIMIT 1) t";

pub async fn list() -> Result<Vec<crate::model::oauth2_app::OAuth2App>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::oauth2_app::OAuth2App>().await?;
    let DbPool::Postgres(p) = pool;
    let p_clone = p.clone();
    let rows: Vec<crate::model::oauth2_app::OAuth2App> =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let mut conn = p_clone
                .get_timeout(Duration::from_secs(2))
                .map_err(|e| anyhow!(e))?;
            let q = diesel::sql_query(SQL_LIST);
            info!("{}", SQL_LIST);
            let r: Vec<AppJsonRow> = q.load(&mut conn).map_err(|e| anyhow!(e))?;
            let models: Vec<crate::model::oauth2_app::OAuth2App> = r
                .into_iter()
                .map(|row| serde_json::from_str::<crate::model::oauth2_app::OAuth2App>(&row.data))
                .collect::<Result<_, _>>()
                .map_err(|e| anyhow!(e))?;
            Ok(models)
        })
        .await??;
    Ok(rows)
}

pub async fn info(id: i64) -> Result<Option<crate::model::oauth2_app::OAuth2App>> {
    let pool = DatabaseManager::get_db_pool::<crate::model::oauth2_app::OAuth2App>().await?;
    let DbPool::Postgres(p) = pool;
    let p_clone = p.clone();
    let rows: Vec<crate::model::oauth2_app::OAuth2App> =
        tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let mut conn = p_clone
                .get_timeout(Duration::from_secs(2))
                .map_err(|e| anyhow!(e))?;
            let q = diesel::sql_query(SQL_INFO).bind::<BigInt, _>(id);
            let expanded = expand_sql_with_binds(SQL_INFO, &[id.to_string()]);
            info!("{}", expanded);
            let r: Vec<AppJsonRow> = q.load(&mut conn).map_err(|e| anyhow!(e))?;
            let models: Vec<crate::model::oauth2_app::OAuth2App> = r
                .into_iter()
                .map(|row| serde_json::from_str::<crate::model::oauth2_app::OAuth2App>(&row.data))
                .collect::<Result<_, _>>()
                .map_err(|e| anyhow!(e))?;
            Ok(models)
        })
        .await??;
    Ok(rows.into_iter().next())
}
