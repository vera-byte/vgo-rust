extern crate v_macros;

#[v_macros::model(table = "my_table", group = "my_group")]
struct Conf {
    name: String,
}

fn main() {
    let _ = Conf::TABLE_NAME;
    let _ = Conf::TABLE_GROUP;
    assert_eq!(Conf::TABLE_NAME, "my_table");
    assert_eq!(Conf::TABLE_GROUP, "my_group");
}