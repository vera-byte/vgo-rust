#[v_macros::base_model]
#[derive(sqlx::FromRow, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[v_macros::model(table_name = "base_sys_conf", group_name = "default", auto_impl = true)]
pub struct BaseSysConf {
    pub c_key: String,
    pub c_value: String,
}
