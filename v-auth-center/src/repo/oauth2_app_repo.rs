use v::db::error::Result;
use std::time::Duration;
use tracing::info;
use v::db::query::QueryPg;

fn expand_sql_with_binds(sql: &str, binds: &[String]) -> String {
    let mut s = sql.to_string();
    for (i, b) in binds.iter().enumerate() {
        let ph = format!("${}", i + 1);
        s = s.replace(&ph, b);
    }
    s
}
// 使用 QueryPg 构建查询 / Use QueryPg builder

// 通过 SQL row_to_json 映射 / Map via SQL row_to_json

const SQL_LIST_INNER: &str = ""; // not used with builder
const SQL_INFO_INNER: &str = ""; // not used with builder

pub async fn list() -> Result<Vec<crate::model::oauth2_app::OAuth2App>> {
    let vals = QueryPg::<crate::model::oauth2_app::OAuth2App>::new()
        .await?
        .order_by("id ASC")
        .fetch_all_json()
        .await?;
    let models = vals
        .into_iter()
        .map(|v| {
            serde_json::from_value::<crate::model::oauth2_app::OAuth2App>(v)
                .map_err(v::db::error::DbError::from)
        })
        .collect::<std::result::Result<Vec<_>, v::db::error::DbError>>()?;
    Ok(models)
}

pub async fn info(id: i64) -> Result<Option<crate::model::oauth2_app::OAuth2App>> {
    let v = QueryPg::<crate::model::oauth2_app::OAuth2App>::new()
        .await?
        .where_eq_json("id", serde_json::json!(id))
        .limit(1)
        .fetch_one_json()
        .await;
    match v {
        Ok(val) => Ok(Some(serde_json::from_value::<
            crate::model::oauth2_app::OAuth2App,
        >(val)?)),
        Err(_) => Ok(None),
    }
}

// 根据client_id查询应用信息
pub async fn info_by_client_id(
    client_id: &str,
) -> Result<Option<crate::model::oauth2_app::OAuth2App>> {
    let v = QueryPg::<crate::model::oauth2_app::OAuth2App>::new()
        .await?
        .where_eq_json("client_id", serde_json::json!(client_id))
        .limit(1)
        .fetch_one_json()
        .await;
    match v {
        Ok(val) => Ok(Some(serde_json::from_value::<
            crate::model::oauth2_app::OAuth2App,
        >(val)?)),
        Err(_) => Ok(None),
    }
}
