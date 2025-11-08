use tokio::runtime::Runtime;

fn main() {
    // 为后续异步认证中心逻辑预留入口
    let rt = Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async move {
        // 示例：启动时打印标识，后续可替换为实际服务初始化
        println!("v-auth-center started");
    });
}