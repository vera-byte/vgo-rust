use crate::VConnectIMServer;
#[cfg(feature = "quic")]
use std::collections::HashMap; // 仅在启用QUIC特性时引入 / Import only when QUIC feature enabled
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
pub struct QuicServer {
    server: Arc<VConnectIMServer>, // 业务服务器 / Business server
    bind_addr: SocketAddr,         // 绑定地址 / Bind address
}

impl QuicServer {
    pub fn new(server: Arc<VConnectIMServer>, bind_addr: SocketAddr) -> Self {
        Self { server, bind_addr }
    }

    #[cfg(feature = "quic")]
    pub async fn start(self) -> JoinHandle<()> {
        use crate::Connection as WsConnection;
        use quiche::{Config, Connection, Header};
        use quiche::{ConnectionId, RecvInfo};
        use rand::RngCore;
        use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

        tokio::spawn(async move {
            let socket = match UdpSocket::bind(self.bind_addr).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("QUIC bind error: {}", e);
                    return;
                }
            };

            let mut config = Config::new(quiche::PROTOCOL_VERSION).expect("quic config");
            // TLS 证书加载 / Load TLS cert
            if let Ok(cm) = v::get_global_config_manager() {
                let cert: String = cm.get_or("quic.tls_cert", String::from("certs/server.crt"));
                let key: String = cm.get_or("quic.tls_key", String::from("certs/server.key"));
                config.load_cert_chain_from_pem_file(&cert).ok();
                config.load_priv_key_from_pem_file(&key).ok();
            }
            config.set_application_protos(&[b"wukong-msg"]).ok();
            config.set_max_idle_timeout(5000);
            config.enable_pacing(true);
            config.set_max_recv_udp_payload_size(1350);
            config.set_max_send_udp_payload_size(1350);

            let mut out = vec![0u8; 65535];
            let mut buf = vec![0u8; 65535];
            let mut connections: HashMap<
                Vec<u8>,
                (Connection, SocketAddr, UnboundedReceiver<Message>, String),
            > = HashMap::new();

            loop {
                match timeout(Duration::from_millis(1000), socket.recv_from(&mut buf)).await {
                    Ok(Ok((len, from))) => {
                        let pkt = &mut buf[..len];
                        let hdr = match Header::from_slice(pkt, quiche::MAX_CONN_ID_LEN) {
                            Ok(h) => h,
                            Err(e) => {
                                tracing::warn!("parse header err: {}", e);
                                continue;
                            }
                        };

                        let conn_key = hdr.dcid.to_vec();
                        let (conn_exists, peer) =
                            if let Some((_, peer, _, _)) = connections.get_mut(&conn_key) {
                                (true, *peer)
                            } else {
                                (false, from)
                            };

                        if !conn_exists {
                            let mut scid_bytes = [0u8; quiche::MAX_CONN_ID_LEN];
                            rand::thread_rng().fill_bytes(&mut scid_bytes);
                            let scid = ConnectionId::from_ref(&scid_bytes);
                            let local = socket.local_addr().unwrap_or(self.bind_addr);
                            let conn = match quiche::accept(&scid, None, local, peer, &mut config) {
                                Ok(c) => c,
                                Err(e) => {
                                    tracing::error!("accept err: {}", e);
                                    continue;
                                }
                            };
                            let (tx, rx) = unbounded_channel::<Message>();
                            // 注册连接到业务映射 / Register connection to business map
                            let client_id = hex::encode(scid_bytes);
                            let ws_conn = WsConnection {
                                client_id: client_id.clone(),
                                uid: None,
                                addr: peer,
                                sender: tx.clone(),
                                last_heartbeat: Arc::new(std::sync::Mutex::new(
                                    std::time::Instant::now(),
                                )),
                            };
                            self.server.connections.insert(client_id.clone(), ws_conn);
                            self.server
                                .quic_conn_count
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            self.server
                                .directory
                                .register_client_location(&client_id, &self.server.node_id);
                            connections.insert(conn_key.clone(), (conn, peer, rx, client_id));

                            // 授权看门狗：连接后必须在deadline内鉴权，否则踢出 / Auth watchdog for QUIC
                            if let Ok(cm) = v::get_global_config_manager() {
                                let auth_deadline_ms: u64 = cm.get_or("auth.deadline_ms", 1000_u64);
                                let server = self.server.clone();
                                let cid = connections
                                    .get(&conn_key)
                                    .map(|(_, _, _, id)| id.clone())
                                    .unwrap_or_default();
                                if !cid.is_empty() {
                                    tokio::spawn(async move {
                                        tokio::time::sleep(std::time::Duration::from_millis(
                                            auth_deadline_ms,
                                        ))
                                        .await;
                                        if let Some(c) = server.connections.get(&cid) {
                                            if c.uid.is_none() {
                                                let _ = server.send_close_message(&cid).await;
                                                server.connections.remove(&cid);
                                                tracing::warn!("disconnecting unauthenticated QUIC client_id={}", cid);
                                            }
                                        }
                                    });
                                }
                            }
                        }

                        if let Some((conn, peer, rx, client_id)) = connections.get_mut(&conn_key) {
                            if *peer != from {
                                self.server
                                    .quic_path_updates
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                tracing::info!(
                                    "QUIC path updated client_id={} old_peer={} new_peer={}",
                                    client_id,
                                    peer,
                                    from
                                );
                                *peer = from;
                            }
                            let pkt_buf = &mut buf[..len];
                            let local = socket.local_addr().unwrap_or(self.bind_addr);
                            let info = RecvInfo {
                                from: *peer,
                                to: local,
                            };
                            if let Err(e) = conn.recv(pkt_buf, info) {
                                tracing::warn!("conn recv err: {}", e);
                                continue;
                            }

                            // 读取应用数据 / Read app data
                            for s in conn.readable() {
                                if let Ok((read, fin)) = conn.stream_recv(s, &mut out) {
                                    let data = &out[..read];
                                    self.server
                                        .quic_stream_recv
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    if let Ok(text) = std::str::from_utf8(data) {
                                        if let Ok(val) =
                                            serde_json::from_str::<serde_json::Value>(text)
                                        {
                                            if let Some(t) =
                                                val.get("type").and_then(|v| v.as_str())
                                            {
                                                if t == "auth" {
                                                    if let Some(uid) = val
                                                        .get("data")
                                                        .and_then(|d| d.get("uid"))
                                                        .and_then(|v| v.as_str())
                                                    {
                                                        let _ = crate::service::auth::apply_auth(
                                                            &self.server,
                                                            client_id,
                                                            uid,
                                                        )
                                                        .await;
                                                        tracing::info!(
                                                            "QUIC auth ok client_id={} uid={}",
                                                            client_id,
                                                            uid
                                                        );
                                                        let ok = serde_json::to_string(&serde_json::json!({"type":"auth_ok","data":{"uid":uid}})).unwrap_or_else(|_|"{\"type\":\"auth_ok\"}".to_string());
                                                        let _ = conn.stream_send(
                                                            0,
                                                            ok.as_bytes(),
                                                            true,
                                                        );
                                                    }
                                                } else {
                                                    let _ = self
                                                        .server
                                                        .handle_incoming_message(
                                                            Message::Text(text.to_string()),
                                                            client_id,
                                                            &self.server.connections,
                                                        )
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                    if fin {
                                        let _ = conn.stream_shutdown(s, quiche::Shutdown::Read, 0);
                                    }
                                }
                            }

                            // 读取Datagram数据 / Read datagram data
                            loop {
                                match conn.dgram_recv(&mut out) {
                                    Ok(len) => {
                                        let data = &out[..len];
                                        self.server
                                            .quic_dgram_recv
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                        if let Ok(text) = std::str::from_utf8(data) {
                                            let _ = self
                                                .server
                                                .handle_incoming_message(
                                                    Message::Text(text.to_string()),
                                                    client_id,
                                                    &self.server.connections,
                                                )
                                                .await;
                                        }
                                    }
                                    Err(quiche::Error::Done) => break,
                                    Err(e) => {
                                        tracing::warn!("conn dgram recv err: {}", e);
                                        break;
                                    }
                                }
                            }

                            // 发送待发送的消息（datagram） / Send pending messages (datagram)
                            while let Ok(msg) = rx.try_recv() {
                                if let Message::Text(text) = msg {
                                    if let Ok(val) =
                                        serde_json::from_str::<serde_json::Value>(&text)
                                    {
                                        let mt =
                                            val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                        match mt {
                                            "message" | "private_message" | "group_message"
                                            | "message_sent" | "pong" => {
                                                let _ = conn.stream_send(1, text.as_bytes(), true);
                                                self.server.quic_stream_sent.fetch_add(
                                                    1,
                                                    std::sync::atomic::Ordering::Relaxed,
                                                );
                                            }
                                            _ => {
                                                let _ = conn.dgram_send(text.as_bytes());
                                                self.server.quic_dgram_sent.fetch_add(
                                                    1,
                                                    std::sync::atomic::Ordering::Relaxed,
                                                );
                                            }
                                        }
                                    } else {
                                        let _ = conn.dgram_send(text.as_bytes());
                                        self.server
                                            .quic_dgram_sent
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    }
                                }
                            }

                            // 发送响应 / Send response
                            loop {
                                match conn.send(&mut out) {
                                    Ok((write, info)) => {
                                        let _ = socket.send_to(&out[..write], info.to).await;
                                    }
                                    Err(quiche::Error::Done) => break,
                                    Err(e) => {
                                        tracing::warn!("conn send err: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("QUIC recv error: {}", e);
                    }
                    Err(_) => {
                        // 定时触发超时处理 / handle timeout
                        for (conn, _peer, _, _) in connections.values_mut() {
                            conn.on_timeout();
                            loop {
                                match conn.send(&mut out) {
                                    Ok((write, info)) => {
                                        let _ = socket.send_to(&out[..write], info.to).await;
                                    }
                                    Err(quiche::Error::Done) => break,
                                    Err(e) => {
                                        tracing::warn!("timeout send err: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    #[cfg(not(feature = "quic"))]
    pub async fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let socket = match UdpSocket::bind(self.bind_addr).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("UDP bind error: {}", e);
                    return;
                }
            };
            let mut buf = vec![0u8; 4096];
            loop {
                match timeout(Duration::from_millis(5000), socket.recv_from(&mut buf)).await {
                    Ok(Ok((n, peer))) => {
                        let payload = &buf[..n];
                        if let Ok(text) = std::str::from_utf8(payload) {
                            let msg = Message::Text(text.to_string());
                            let _ = self
                                .server
                                .handle_incoming_message(
                                    msg,
                                    &peer.to_string(),
                                    &self.server.connections,
                                )
                                .await;
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("UDP recv error: {}", e);
                    }
                    Err(_) => {
                        // 超时轮询 / poll on timeout
                    }
                }
            }
        })
    }
}
