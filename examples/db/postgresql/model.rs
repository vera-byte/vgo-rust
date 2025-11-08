#[v_macros::base_model]
#[v_macros::model(table_name = "base_sys_conf", group_name = "default", auto_impl = true)]
pub struct BaseSysConf {
    pub name: String,
    pub value: String,
}

#[v_macros::base_model]
#[v_macros::model(table_name = "base_sys_conf2", group_name = "test", auto_impl = true)]
pub struct BaseSysConf2 {
    pub name: String,
    pub value: String,
}
