use rand::Rng;
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // 代理监听与目标地址 / Proxy listen and target address
    let listen: SocketAddr = std::env::var("LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:5300".to_string())
        .parse()
        .expect("LISTEN addr");
    let target: SocketAddr = std::env::var("TARGET")
        .unwrap_or_else(|_| "127.0.0.1:5201".to_string())
        .parse()
        .expect("TARGET addr");
    let drop_p: f64 = std::env::var("DROP_P").ok().and_then(|v| v.parse().ok()).unwrap_or(0.2);
    let jitter_ms: u64 = std::env::var("JITTER_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(100);
    let delay_ms: u64 = std::env::var("DELAY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(50);

    let sock = tokio::net::UdpSocket::bind(listen).await.unwrap();
    println!("weak proxy listening at {} -> target {}", listen, target);
    let mut buf = vec![0u8; 65535];
    let mut last_client: Option<SocketAddr> = None;
    loop {
        if let Ok((len, from)) = sock.recv_from(&mut buf).await {
            // 选择方向 / decide direction
            let to_server = from != target;
            if to_server { last_client = Some(from); }
            let dest = if to_server { target } else { last_client.unwrap_or(from) };
            // 丢包 / drop
            let r: f64 = rand::thread_rng().gen();
            if r < drop_p { continue; }
            // 抖动与延迟 / jitter and delay
            let jitter = rand::thread_rng().gen_range(0..=jitter_ms);
            sleep(Duration::from_millis(delay_ms + jitter)).await;
            let _ = sock.send_to(&buf[..len], dest).await;
        }
    }
}

