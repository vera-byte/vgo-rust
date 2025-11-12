use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

// 运行时创建表，示例使用内存数据库
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
    let mut conn = SqliteConnection::establish(":memory:").expect("无法连接到内存 SQLite");

    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS base_sys_conf (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value TEXT NOT NULL
        )",
    )
    .execute(&mut conn)
    .expect("创建表失败");

    let rows = vec![
        NewBaseSysConf {
            name: "site_name",
            value: "vgo",
        },
        NewBaseSysConf {
            name: "theme",
            value: "dark",
        },
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
