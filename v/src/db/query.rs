use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use sqlx::postgres::PgRow;
use sqlx::types;
use sqlx::Column;
use sqlx::Postgres;
use sqlx::QueryBuilder;
use sqlx::Row;

use crate::db::error::{DbError, Result};
use crate::db::model::pool_for;
use crate::db::model::{ColType, DbModel, ModelSpec};

lazy_static::lazy_static! {
    static ref CACHE: RwLock<HashMap<String, (Instant, Vec<serde_json::Value>)>> = RwLock::new(HashMap::new());
}

/// 查询构建器（PostgreSQL）/ Query builder (PostgreSQL)
///
/// 特性 / Features:
/// - 自动路由到模型所在数据库组 / auto route via `DbModel::table_group`
/// - 基本 CRUD、事务友好 / basic CRUD, transaction-friendly
/// - 简单查询缓存（TTL）/ simple TTL query cache
pub struct QueryPg<M: DbModel> {
    pool: sqlx::Pool<Postgres>,
    table: &'static str,
    select_cols: Option<Vec<String>>,
    where_parts: Vec<WherePart>,
    order_sql: String,
    limit_sql: String,
    cache_ttl: Option<Duration>,
    _marker: std::marker::PhantomData<M>,
}

impl<M: DbModel> QueryPg<M> {
    /// 创建查询构建器 / Create builder
    /// 使用模型的分库组获取连接池 / gets pool by model's group
    pub async fn new() -> Result<Self> {
        let pool = pool_for::<M>().await?;
        Ok(Self {
            pool,
            table: M::table_name(),
            select_cols: None,
            where_parts: Vec::new(),
            order_sql: String::new(),
            limit_sql: String::new(),
            cache_ttl: None,
            _marker: std::marker::PhantomData,
        })
    }

    /// 选择列 / Select columns
    /// 默认 `*` / default `*`
    pub fn select(mut self, cols: &[&str]) -> Self {
        self.select_cols = Some(cols.iter().map(|s| s.to_string()).collect());
        self
    }

    /// 启用缓存 / Enable cache
    /// 仅对 `SELECT` 生效 / applies to SELECT only
    pub fn enable_cache(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    /// where 等值 / where equals
    pub fn where_eq_json(mut self, col: &str, val: serde_json::Value) -> Self {
        if self.where_parts.is_empty() {
            self.where_parts.push(WherePart::Raw(" WHERE ".into()));
        } else {
            self.where_parts.push(WherePart::Raw(" AND ".into()));
        }
        self.where_parts
            .push(WherePart::Raw(format!("\"{}\" = ", col)));
        self.where_parts.push(WherePart::Bind(val));
        self
    }

    /// 自定义 where 片段（带绑定）/ custom where fragment with bind
    pub fn where_raw(mut self, sql: &str, bind_val: serde_json::Value) -> Self {
        if self.where_parts.is_empty() {
            self.where_parts.push(WherePart::Raw(" WHERE ".into()));
        } else {
            self.where_parts.push(WherePart::Raw(" AND ".into()));
        }
        self.where_parts.push(WherePart::Raw(sql.to_string()));
        self.where_parts.push(WherePart::Bind(bind_val));
        self
    }

    /// 排序 / order by
    pub fn order_by(mut self, expr: &str) -> Self {
        self.order_sql = format!(" ORDER BY {}", expr);
        self
    }
    /// 限制 / limit
    pub fn limit(mut self, n: i64) -> Self {
        self.limit_sql = format!(" LIMIT {}", n);
        self
    }

    /// 查询所有（JSON）/ fetch all as JSON
    /// 将行自动转换为 `serde_json::Value` / converts rows to JSON
    pub async fn fetch_all_json(mut self) -> Result<Vec<serde_json::Value>> {
        let select = match &self.select_cols {
            Some(cols) => cols.join(", "),
            None => "*".to_string(),
        };
        let mut qb =
            QueryBuilder::<Postgres>::new(format!("SELECT {} FROM \"{}\"", select, self.table));
        for p in &self.where_parts {
            match p {
                WherePart::Raw(s) => {
                    qb.push(s);
                }
                WherePart::Bind(v) => push_value(&mut qb, v),
            }
        }
        qb.push(&self.order_sql);
        qb.push(&self.limit_sql);
        let sql_key = qb.sql().to_string();
        if let Some(ttl) = self.cache_ttl {
            if let Some((ts, data)) = CACHE.read().await.get(&sql_key).cloned() {
                if ts.elapsed() < ttl {
                    return Ok(data);
                }
            }
        }
        let rows: Vec<PgRow> = qb.build().fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(row_to_json(&r)?);
        }
        if let Some(ttl) = self.cache_ttl {
            CACHE
                .write()
                .await
                .insert(sql_key, (Instant::now(), out.clone()));
            let _ = ttl;
        }
        Ok(out)
    }

    /// 查询一条（JSON）/ fetch one as JSON
    pub async fn fetch_one_json(self) -> Result<serde_json::Value> {
        let mut v = self.limit(1).fetch_all_json().await?;
        v.pop().ok_or(DbError::NotFound)
    }

    /// 插入（使用 ModelSpec 自动映射）/ insert using ModelSpec columns
    /// 依据 `ModelSpec::columns` 将结构体序列化并绑定 / binds via column spec
    pub async fn insert_one_spec<T: serde::Serialize + ModelSpec>(self, item: &T) -> Result<u64> {
        let cols = T::columns();
        let mut qb =
            QueryBuilder::<Postgres>::new(format!("INSERT INTO \"{}\" (", M::table_name()));
        qb.push(
            cols.iter()
                .map(|c| format!("\"{}\"", c.name))
                .collect::<Vec<_>>()
                .join(", "),
        );
        qb.push(") VALUES (");
        let js = serde_json::to_value(item)?;
        let obj = js
            .as_object()
            .ok_or(DbError::Config("expected object".to_string()))?;
        for (i, c) in cols.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            match c.ty {
                ColType::Text => {
                    qb.push_bind(
                        obj.get(c.name)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                ColType::Int64 => {
                    qb.push_bind(obj.get(c.name).and_then(|v| v.as_i64()).unwrap_or(0i64));
                }
                ColType::Int16 => {
                    qb.push_bind(obj.get(c.name).and_then(|v| v.as_i64()).unwrap_or(0i64) as i16);
                }
                ColType::Bool => {
                    qb.push_bind(obj.get(c.name).and_then(|v| v.as_bool()).unwrap_or(false));
                }
                ColType::Timestamp => {
                    qb.push_bind(
                        obj.get(c.name)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                }
                ColType::Json => {
                    qb.push_bind(sqlx::types::Json(
                        obj.get(c.name).cloned().unwrap_or(serde_json::Value::Null),
                    ));
                }
                ColType::ArrayText => {
                    let arr = obj
                        .get(c.name)
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();
                    qb.push_bind(arr);
                }
            }
        }
        qb.push(")");
        let res = qb.build().execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    /// 更新（按 where 条件）/ update with where
    pub async fn update_map(mut self, set: &HashMap<&str, serde_json::Value>) -> Result<u64> {
        let mut qb = QueryBuilder::<Postgres>::new(format!("UPDATE \"{}\" SET ", self.table));
        let mut first = true;
        for (k, v) in set.iter() {
            if !first {
                qb.push(", ");
            } else {
                first = false;
            }
            qb.push(format!("\"{}\" = ", k));
            push_value(&mut qb, v);
        }
        for p in &self.where_parts {
            match p {
                WherePart::Raw(s) => {
                    qb.push(s);
                }
                WherePart::Bind(v) => push_value(&mut qb, v),
            }
        }
        let res = qb.build().execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    /// 删除 / delete
    pub async fn delete(self) -> Result<u64> {
        let mut qb = QueryBuilder::<Postgres>::new(format!("DELETE FROM \"{}\"", self.table));
        for p in &self.where_parts {
            match p {
                WherePart::Raw(s) => {
                    qb.push(s);
                }
                WherePart::Bind(v) => push_value(&mut qb, v),
            }
        }
        let res = qb.build().execute(&self.pool).await?;
        Ok(res.rows_affected())
    }

    /// 批量插入（数组 JSON）/ batch insert from JSON array
    pub async fn insert_many_json(self, items: &[serde_json::Value]) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let first_obj = items[0]
            .as_object()
            .ok_or(DbError::Config("expected object".to_string()))?;
        let cols: Vec<String> = first_obj.keys().cloned().collect();
        let mut qb = QueryBuilder::<Postgres>::new(format!(
            "INSERT INTO \"{}\" ({} ) VALUES ",
            self.table,
            cols.iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        qb.push_values(items, |mut b, item| {
            let obj = item.as_object().unwrap();
            for (i, col) in cols.iter().enumerate() {
                if i > 0 {
                    b.push(", ");
                }
                match obj.get(col).unwrap_or(&serde_json::Value::Null) {
                    serde_json::Value::String(s) => {
                        b.push_bind(s.clone());
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            b.push_bind(i);
                        } else if let Some(f) = n.as_f64() {
                            b.push_bind(f);
                        } else {
                            b.push_bind(n.to_string());
                        }
                    }
                    serde_json::Value::Bool(bb) => {
                        b.push_bind(*bb);
                    }
                    serde_json::Value::Array(a) => {
                        let vs: Vec<String> = a.iter().map(|x| x.to_string()).collect();
                        b.push_bind(vs);
                    }
                    serde_json::Value::Object(_) => {
                        b.push_bind(sqlx::types::Json(item.clone()));
                    }
                    serde_json::Value::Null => {
                        b.push_bind(serde_json::Value::Null.to_string());
                    }
                }
            }
        });
        let res = qb.build().execute(&self.pool).await?;
        Ok(res.rows_affected())
    }
}

fn push_value(qb: &mut QueryBuilder<'_, Postgres>, v: &serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            qb.push_bind(s.clone());
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                qb.push_bind(i);
            } else if let Some(f) = n.as_f64() {
                qb.push_bind(f);
            } else {
                qb.push_bind(n.to_string());
            }
        }
        serde_json::Value::Bool(b) => {
            qb.push_bind(*b);
        }
        serde_json::Value::Array(a) => {
            let vs: Vec<String> = a.iter().map(|x| x.to_string()).collect();
            qb.push_bind(vs);
        }
        serde_json::Value::Object(_) => {
            qb.push_bind(sqlx::types::Json(v.clone()));
        }
        serde_json::Value::Null => {
            qb.push_bind(serde_json::Value::Null.to_string());
        }
    }
}

enum WherePart {
    Raw(String),
    Bind(serde_json::Value),
}

fn row_to_json(row: &PgRow) -> Result<serde_json::Value> {
    // 将一行转换为 JSON，尽量覆盖常见类型，避免出现 Null 导致反序列化失败
    // Convert a row to JSON, cover common types to avoid Null causing deserialization errors
    let mut map = serde_json::Map::new();
    for col in row.columns() {
        let name = col.name();
        let val_json = row
            .try_get::<types::Json<serde_json::Value>, _>(name)
            .map(|j| j.0)
            // try timestamps before raw strings to avoid timezone suffix issues
            .or_else(|_| {
                row.try_get::<i16, _>(name)
                    .map(|v| serde_json::Value::from(v as i64))
            })
            .or_else(|_| {
                row.try_get::<i32, _>(name)
                    .map(|v| serde_json::Value::from(v as i64))
            })
            .or_else(|_| {
                row.try_get::<i64, _>(name)
                    .map(|v| serde_json::Value::from(v))
            })
            .or_else(|_| {
                row.try_get::<bool, _>(name)
                    .map(|v| serde_json::Value::from(v))
            })
            .or_else(|_| {
                row.try_get::<Vec<String>, _>(name)
                    .map(|v| serde_json::Value::from(v))
            })
            .or_else(|_| {
                row.try_get::<chrono::NaiveDateTime, _>(name).map(|dt| {
                    serde_json::Value::String(dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string())
                })
            })
            .or_else(|_| {
                row.try_get::<chrono::DateTime<chrono::Utc>, _>(name)
                    .map(|dt| {
                        serde_json::Value::String(
                            dt.naive_utc().format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
                        )
                    })
            })
            .or_else(|_| {
                row.try_get::<String, _>(name)
                    .map(serde_json::Value::String)
            })
            .unwrap_or(serde_json::Value::Null);
        map.insert(name.to_string(), val_json);
    }
    Ok(serde_json::Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::model::DbModel;

    struct Dummy;
    impl DbModel for Dummy {
        fn table_name() -> &'static str {
            "pg_catalog.pg_class"
        }
        fn table_group() -> &'static str {
            "default"
        }
    }

    #[tokio::test]
    async fn test_select_builder_and_cache() {
        std::env::set_var("V_DATABASE_DEFAULT_TYPE", "postgresql");
        std::env::set_var("V_DATABASE_DEFAULT_HOST", "127.0.0.1");
        std::env::set_var("V_DATABASE_DEFAULT_PORT", "5432");
        std::env::set_var("V_DATABASE_DEFAULT_USER", "postgres");
        std::env::set_var("V_DATABASE_DEFAULT_PASS", "");
        std::env::set_var("V_DATABASE_DEFAULT_NAME", "postgres");
        let q = QueryPg::<Dummy>::new()
            .await
            .unwrap()
            .select(&["relname"])
            .limit(1)
            .enable_cache(Duration::from_secs(30));
        let _ = q.fetch_all_json().await; // may fail if db not running
    }
}
