extern crate v_macros;
extern crate chrono;
extern crate serde_json;

#[v_macros::base_model]
struct Conf {
    name: String,
    value: String,
}

fn main() {
    let c = Conf {
        name: "n".to_string(),
        value: "v".to_string(),
        id: 42,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    };

    let json = serde_json::to_string(&c).unwrap();
    let back: Conf = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, 42);
}