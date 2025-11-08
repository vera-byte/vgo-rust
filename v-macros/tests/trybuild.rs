#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_base_model.rs");
    t.pass("tests/ui/pass_model.rs");
}