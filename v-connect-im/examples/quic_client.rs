use rand::RngCore;
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() {
    // 服务端地址可配置 / Server address configurable
    let peer_env = std::env::var("PEER").unwrap_or_else(|_| "127.0.0.1:5201".to_string());
    let peer: SocketAddr = peer_env.parse().unwrap();
    // 绑定本地UDP套接字 / Bind local UDP socket
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let local = socket.local_addr().unwrap();

    // 配置QUIC客户端 / Configure QUIC client
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.set_application_protos(&[b"wukong-msg"]).unwrap();
    config.set_max_idle_timeout(5000);
    config.enable_pacing(true);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);
    config.set_disable_active_migration(false); // 允许迁移 / allow migration
    config.verify_peer(false); // 示例关闭证书校验 / disable cert verify for demo

    // 生成连接ID / Generate connection ID
    let mut scid_bytes = [0u8; quiche::MAX_CONN_ID_LEN];
    rand::thread_rng().fill_bytes(&mut scid_bytes);
    let scid = quiche::ConnectionId::from_ref(&scid_bytes);
    let mut conn = quiche::connect(Some("localhost"), &scid, local, peer, &mut config).unwrap();

    let mut out = vec![0u8; 65535];
    let mut buf = vec![0u8; 65535];

    // 初始握手发送 / initial flight
    if let Ok((write, info)) = conn.send(&mut out) {
        socket.send_to(&out[..write], info.to).await.unwrap();
    }

    let mut sent_auth = false; // 是否已发送认证 / auth sent
    let mut sent_ping = false; // 是否已发送ping / ping sent

    let mut loops = 0u32;
    loop {
        // 接收服务端数据（带超时）/ recv server data with timeout
        if let Ok(Ok((len, from))) =
            timeout(Duration::from_millis(300), socket.recv_from(&mut buf)).await
        {
            let info = quiche::RecvInfo { from, to: local };
            let _ = conn.recv(&mut buf[..len], info);
        }

        // 握手完成后先发送认证首帧（stream 0）/ after handshake, send auth on stream 0
        if conn.is_established() && !sent_auth {
            let auth = serde_json::to_string(&serde_json::json!({
                "type":"auth",
                "data":{"uid":"uClient","token":"dummy"}
            }))
            .unwrap();
            let _ = conn.stream_send(0, auth.as_bytes(), true);
            sent_auth = true;
        }

        // 认证后发送ping（stream 4）/ after auth, send ping on stream 4
        if conn.is_established() && sent_auth && !sent_ping {
            let payload = serde_json::to_string(&serde_json::json!({
                "type":"ping",
                "data":{}
            }))
            .unwrap();
            let _ = conn.stream_send(4, payload.as_bytes(), true);
            sent_ping = true;
        }

        // 读取响应 / read response
        for s in conn.readable() {
            let mut stream_buf = [0u8; 4096];
            if let Ok((read, fin)) = conn.stream_recv(s, &mut stream_buf) {
                let data = &stream_buf[..read];
                if let Ok(txt) = std::str::from_utf8(data) {
                    println!("recv: {}", txt);
                }
                if fin {
                    let _ = conn.stream_shutdown(s, quiche::Shutdown::Read, 0);
                }
            }
        }

        // 发送待发送数据 / send pending
        loop {
            match conn.send(&mut out) {
                Ok((write, info)) => {
                    socket.send_to(&out[..write], info.to).await.unwrap();
                }
                Err(quiche::Error::Done) => break,
                Err(e) => {
                    eprintln!("send err: {}", e);
                    break;
                }
            }
        }

        loops += 1;
        if sent_auth && sent_ping && loops > 5 {
            break;
        }
    }

    // 模拟迁移：更换本地端口并继续发送 / simulate migration: change local port and continue
    let new_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let local_new = new_socket.local_addr().unwrap();
    // 发送一个空的pending包刷新路径 / send pending to refresh path
    if let Ok((write, info)) = conn.send(&mut out) {
        new_socket.send_to(&out[..write], info.to).await.unwrap();
    }
    // 发送迁移后的ping / send migrated ping
    let payload2 = serde_json::to_string(&serde_json::json!({
        "type":"ping",
        "data":{"migrated":true}
    }))
    .unwrap();
    let _ = conn.stream_send(6, payload2.as_bytes(), true);
    // 发送pending / send pending
    loop {
        match conn.send(&mut out) {
            Ok((write, info)) => {
                new_socket.send_to(&out[..write], info.to).await.unwrap();
            }
            Err(quiche::Error::Done) => break,
            Err(e) => {
                eprintln!("send err after migrate: {}", e);
                break;
            }
        }
    }
    // 接收迁移后的响应 / recv migrated response
    if let Ok(Ok((len, from))) =
        timeout(Duration::from_millis(500), new_socket.recv_from(&mut buf)).await
    {
        let info = quiche::RecvInfo {
            from,
            to: local_new,
        };
        let _ = conn.recv(&mut buf[..len], info);
        for s in conn.readable() {
            let mut stream_buf = [0u8; 4096];
            if let Ok((read, fin)) = conn.stream_recv(s, &mut stream_buf) {
                let data = &stream_buf[..read];
                if let Ok(txt) = std::str::from_utf8(data) {
                    println!("recv after migrate: {}", txt);
                }
                if fin {
                    let _ = conn.stream_shutdown(s, quiche::Shutdown::Read, 0);
                }
            }
        }
    }
}
