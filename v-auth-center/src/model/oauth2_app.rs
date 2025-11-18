// OAuth2 应用模型 / OAuth2 Application model
// 说明: 业务模型字段与数据库结构统一为蛇形命名（snake_case）
// Description: Business model fields aligned with database schema using snake_case

use serde::{Deserialize, Serialize};
use diesel::deserialize::QueryableByName;
use diesel::sql_types::{BigInt, Bool, Jsonb, SmallInt, Text, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, QueryableByName)]
pub struct OAuth2App {
    #[diesel(sql_type = BigInt)]
    pub id: i64,
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>)]
    pub tenant_id: Option<i64>,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub client_id: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub client_secret: Option<String>,
    #[diesel(sql_type = SmallInt)]
    pub confidentiality: i16,
    #[diesel(sql_type = Bool)]
    pub first_party: bool,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = diesel::sql_types::Nullable<Timestamp>)]
    pub revoked_at: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::Array<Text>)]
    pub redirect_uris: Vec<String>,
    #[diesel(sql_type = diesel::sql_types::Array<Text>)]
    pub grant_types: Vec<String>,
    #[diesel(sql_type = diesel::sql_types::Array<Text>)]
    pub response_types: Vec<String>,
    #[diesel(sql_type = diesel::sql_types::Array<Text>)]
    pub scopes: Vec<String>,
    #[diesel(sql_type = Text)]
    pub token_auth_method: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub logo_url: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub website_url: Option<String>,
    #[diesel(sql_type = Jsonb)]
    pub contacts: serde_json::Value,
    #[diesel(sql_type = Jsonb)]
    pub metadata: serde_json::Value,
    #[diesel(sql_type = Timestamp)]
    pub created_at: chrono::NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    pub updated_at: chrono::NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Nullable<Timestamp>)]
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

pub const TABLE_NAME: &str = "oauth2_app";
pub const TABLE_GROUP: &str = "default";
