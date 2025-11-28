// build.rs (delegation + proto)
// English: This build script delegates code generation to the shared generator in v::comm::api and compiles proto files.
// 中文：此构建脚本把代码生成委托给 v::comm::api 中的通用生成器，并编译 proto 文件。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/api");
    v::comm::generator::run_for_auth_center().expect("code generation failed");

    // 编译 proto 文件 / Compile proto files
    // 获取 manifest 目录（项目根目录）/ Get manifest directory (project root)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| std::path::PathBuf::from(d))
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    let proto_dir = manifest_dir.join("proto");
    let proto_file = proto_dir.join("plugin.proto");

    if !proto_file.exists() {
        eprintln!(
            "Warning: proto file not found: {}, skipping proto compilation",
            proto_file.display()
        );
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", proto_file.display());

    let out_dir = manifest_dir.join("src").join("proto");
    std::fs::create_dir_all(&out_dir)?;

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .out_dir(&out_dir)
        .compile(&[&proto_file], &[&proto_dir])?;

    Ok(())
}
