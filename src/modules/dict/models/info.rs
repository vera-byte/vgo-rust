// use crate::schema::dict_info; // 暂时注释掉，避免 schema 导入问题
// use diesel::prelude::*; // 暂时注释掉，避免未使用警告


/*
#[derive(Debug, Clone, Serialize, Deserialize)]
// #[diesel(table_name = dict_info)] // 暂时注释掉，避免 schema 问题
pub struct DictInfo {
    /// 主键ID (继承自 BaseEntity)
    pub id: Option<i64>,

    /// 租户ID (继承自 BaseEntity)
    pub tenant_id: Option<String>,

    /// 创建时间 (继承自 BaseEntity)
    pub create_time: Option<std::time::SystemTime>,

    /// 更新时间 (继承自 BaseEntity)
    pub update_time: Option<std::time::SystemTime>,

    /// 字典类型ID
    pub type_id: i32,

    /// 字典项名称
    pub name: String,

    /// 字典项值
    pub value: String,

    /// 排序号
    pub order_num: i32,

    /// 备注信息
    pub remark: Option<String>,

    /// 父级ID，用于构建树形结构
    pub parent_id: Option<i32>,
}

impl DictInfo {
    /// 从 BaseEntity 创建 DictInfo
    /// 这个方法演示了如何将 BaseEntity 的字段复制到 DictInfo 中
    pub fn from_base_entity(
        base: BaseEntity,
        type_id: i32,
        name: String,
        value: String,
        order_num: i32,
    ) -> Self {
        Self {
            id: base.id,
            tenant_id: base.tenant_id,
            create_time: base.create_time,
            update_time: base.update_time,
            type_id,
            name,
            value,
            order_num,
            remark: None,
            parent_id: None,
        }
    }

    /// 获取 BaseEntity 部分
    /// 这个方法演示了如何从 DictInfo 中提取 BaseEntity 字段
    pub fn to_base_entity(&self) -> BaseEntity {
        BaseEntity {
            id: self.id,
            tenant_id: self.tenant_id.clone(),
            create_time: self.create_time,
            update_time: self.update_time,
        }
    }

    /// 更新基础字段的时间戳
    pub fn update_timestamps(&mut self) {
        let now = std::time::SystemTime::now();
        if self.create_time.is_none() {
            self.create_time = Some(now);
        }
        self.update_time = Some(now);
    }
}
*/
