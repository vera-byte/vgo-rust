use v::db::error::Result;
use v::db::query::QueryPg;
const SQL_LIST_INNER: &str = ""; // not used with builder
const SQL_INFO_INNER: &str = ""; // not used with builder

pub async fn list() -> Result<Vec<crate::model::user::User>> {
    let vals = QueryPg::<crate::model::user::User>::new()
        .await?
        .order_by("id ASC")
        .fetch_all_json()
        .await?;
    let models = vals
        .into_iter()
        .map(|v| {
            serde_json::from_value::<crate::model::user::User>(v)
                .map_err(v::db::error::DbError::from)
        })
        .collect::<std::result::Result<Vec<_>, v::db::error::DbError>>()?;
    Ok(models)
}

pub async fn info(id: i64) -> Result<Option<crate::model::user::User>> {
    let v = QueryPg::<crate::model::user::User>::new()
        .await?
        .where_eq_json("id", serde_json::json!(id))
        .limit(1)
        .fetch_one_json()
        .await;
    match v {
        Ok(val) => Ok(Some(serde_json::from_value::<crate::model::user::User>(val)?)),
        Err(_) => Ok(None),
    }
}
use std::time::Duration;
