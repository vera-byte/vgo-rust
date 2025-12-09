// 构建脚本：生成 Protobuf 代码 / Build script: Generate Protobuf code
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "protobuf")]
    {
        use std::path::PathBuf;

        // 创建输出目录 / Create output directory
        let out_dir = PathBuf::from("src/plugin/proto");
        std::fs::create_dir_all(&out_dir)?;

        // 编译 proto 文件 / Compile proto files
        let proto_files = vec![
            "proto/base.proto",            // 基础协议（握手、事件）
            "proto/storage/storage.proto", // 存储插件协议
            "proto/auth/auth.proto",       // 认证插件协议
            "proto/gateway/gateway.proto", // 网关插件协议
        ];

        prost_build::Config::new()
            .out_dir(&out_dir)
            .compile_protos(&proto_files, &["proto/"])?;

        // 监听文件变化 / Watch file changes
        for proto_file in &proto_files {
            println!("cargo:rerun-if-changed={}", proto_file);
        }

        println!("cargo:warning=Protobuf code generated successfully");
    }

    Ok(())
}
