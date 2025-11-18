// 用户业务模型 / User business model
use diesel::deserialize::QueryableByName;
use diesel::sql_types::{BigInt, Bool, Date, Integer, SmallInt, Text, Timestamp};
use serde::{Deserialize, Serialize};
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, QueryableByName)]
pub struct User {
    #[diesel(sql_type = BigInt)]
    pub id: i64,
    #[diesel(sql_type = Timestamp)]
    pub created_at: chrono::NaiveDateTime,
    #[diesel(sql_type = Timestamp)]
    pub updated_at: chrono::NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Nullable<Integer>)]
    pub tenant_id: Option<i32>,
    #[diesel(sql_type = Text)]
    pub username: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub email: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub mobile: Option<String>,
    #[diesel(sql_type = Text)]
    pub password_hash: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub real_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub nick_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub avatar_url: Option<String>,
    #[diesel(sql_type = SmallInt)]
    pub gender: i16,
    #[diesel(sql_type = diesel::sql_types::Nullable<Date>)]
    pub birthday: Option<chrono::NaiveDate>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub id_card: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub country: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub province: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub city: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub district: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub address: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub postal_code: Option<String>,
    #[diesel(sql_type = SmallInt)]
    pub status: i16,
    #[diesel(sql_type = Bool)]
    pub is_email_verified: bool,
    #[diesel(sql_type = Bool)]
    pub is_mobile_verified: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<Timestamp>)]
    pub last_login_time: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub last_login_ip: Option<String>,
    #[diesel(sql_type = Integer)]
    pub login_count: i32,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub source: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub invite_code: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>)]
    pub referrer_id: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    pub remark: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Timestamp>)]
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

pub const TABLE_NAME: &str = "user";
pub const TABLE_GROUP: &str = "default";
