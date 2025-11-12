use diesel::prelude::*;
use diesel::mysql::MysqlConnection;
use dotenvy::dotenv;
use std::env;

diesel::table! {
    base_sys_conf (id) {
        id -> BigInt,
        name -> Text,
        value -> Text,
    }
}

#[derive(Queryable, Debug)]
struct BaseSysConf {
    pub id: i64,
    pub name: String,
    pub value: String,
}

#[derive(Insertable)]
#[diesel(table_name = base_sys_conf)]
struct NewBaseSysConf<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

fn main() {
    dotenv().ok();
    let url = env::var("DATABASE_URL").expect("请设置 DATABASE_URL 环境变量，如 mysql://user:pass@127.0.0.1:3306/vgo");
    let mut conn = MysqlConnection::establish(&url).expect("无法连接到 MySQL");

    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS base_sys_conf (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            name TEXT NOT NULL,
            value TEXT NOT NULL
        ) ENGINE=InnoDB",
    )
    .execute(&mut conn)
    .expect("创建表失败");

    let rows = vec![
        NewBaseSysConf { name: "site_name", value: "vgo" },
        NewBaseSysConf { name: "theme", value: "dark" },
    ];

    diesel::insert_into(base_sys_conf::table)
        .values(&rows)
        .execute(&mut conn)
        .expect("插入失败");

    let list: Vec<BaseSysConf> = base_sys_conf::table
        .order(base_sys_conf::id.asc())
        .load(&mut conn)
        .expect("查询失败");

    println!("查询结果: {:?}", list);
}