pub fn handle_version_command() {
    println!("vgo-rust v{}", env!("CARGO_PKG_VERSION"));
    println!("基于 Rust 的 Web 应用服务器");
    println!("作者: 金书记");
}
