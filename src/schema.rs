// @generated automatically by Diesel CLI.

diesel::table! {
    dict_info (id) {
        id -> Nullable<Bigint>,
        tenant_id -> Nullable<Text>,
        create_time -> Nullable<Timestamp>,
        update_time -> Nullable<Timestamp>,
        type_id -> Integer,
        name -> Text,
        value -> Text,
        order_num -> Integer,
        remark -> Nullable<Text>,
        parent_id -> Nullable<Integer>,
    }
}

// 定义 BaseEntity 的字段，用于 flatten
diesel::table! {
    base (id) {
        id -> Nullable<Bigint>,
        tenant_id -> Nullable<Text>,
        create_time -> Nullable<Timestamp>,
        update_time -> Nullable<Timestamp>,
    }
}