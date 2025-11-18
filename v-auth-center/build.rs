// build.rs (delegation)
// English: This build script delegates code generation to the shared generator in v::comm::api.
// 中文：此构建脚本把代码生成委托给 v::comm::api 中的通用生成器。

fn main() {
    println!("cargo:rerun-if-changed=src/api");
    println!("cargo:rerun-if-changed=src/model");
    v::comm::api::run_for_auth_center().expect("code generation failed");
}

