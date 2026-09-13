//! P2P — port từ src/p2p/P2P.js + PeerManager.js.
//! Mỗi kết nối WebSocket = 1 tokio task; outgoing qua unbounded channel.
//! Message handlers ở handler.rs, sync ở sync.rs.

pub mod handler;
pub mod messages;
pub mod sync;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config;
use crate::node::NodeHandle;
use crate::util;

use handler::connection_task;

/// addr -> connection ra peer đó.
/// `canonical`: addr đã là `host:listenPort` ổn định (peer quảng bá listenPort
/// qua handshake) — chỉ addr canonical mới được broadcast trong PEERS, tránh
/// lan truyền ephemeral port không connect lại được.
#[derive(Clone)]
pub struct PeerConn {
    pub tx: mpsc::UnboundedSender<String>,
    pub canonical: bool,
}

impl PeerConn {
    /// Entry placeholder trong lúc connect tới peer discovery — chiếm key
    /// để race discovery không connect đôi.
    pub fn reserving() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self {
            tx,
            canonical: false,
        }
    }
}

/// addr -> connection
pub type PeerMap = Arc<StdMutex<HashMap<String, PeerConn>>>;

pub struct P2P {
    pub peers: PeerMap,
    pub node: NodeHandle,
    /// Handle của accept-loop để đóng server
    server_task: StdMutex<Option<JoinHandle<()>>>,
    pub server_port: StdMutex<Option<u16>>,
}

impl P2P {
    pub fn new(node: NodeHandle) -> Arc<Self> {
        Arc::new(Self {
            peers: Arc::new(StdMutex::new(HashMap::new())),
            node,
            server_task: StdMutex::new(None),
            server_port: StdMutex::new(None),
        })
    }

    /// Mở P2P server trên port
    pub async fn start_server(self: &Arc<Self>, port: u16) {
        if self.server_task.lock().unwrap().is_some() {
            println!("[P2P] ⚠ Server already running. Close it first.");
            return;
        }
        let listener = match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[P2P] ✗ Port {port} is already in use! ({e})");
                return;
            }
        };
        // port 0 (test) -> lấy port thật mà OS cấp
        let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
        *self.server_port.lock().unwrap() = Some(actual_port);

        let node = self.node.clone();
        let peers = self.peers.clone();
        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let addr = stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => {
                        println!("[P2P] New peer connected from {addr}");
                        tokio::spawn(connection_task(
                            node.clone(),
                            peers.clone(),
                            addr,
                            ws,
                            Some(actual_port),
                            false,
                        ));
                    }
                    Err(e) => eprintln!("[P2P] ✗ WebSocket handshake failed: {e}"),
                }
            }
        });
        *self.server_task.lock().unwrap() = Some(handle);
        println!("[P2P] ✓ P2P server running on port {actual_port}");
    }

    /// Đóng P2P server
    pub fn close_server(&self) {
        if let Some(task) = self.server_task.lock().unwrap().take() {
            task.abort();
            *self.server_port.lock().unwrap() = None;
            println!("[P2P] ✓ P2P server closed successfully.");
        } else {
            println!("[P2P] No server is running.");
        }
    }

    /// Kết nối tới peer ws://host:port
    pub async fn connect_to_peer(self: &Arc<Self>, host: &str, port: u16) {
        // Kiểm tra tự kết nối
        let self_port = *self.server_port.lock().unwrap();
        if self_port == Some(port) && util::is_localhost(host) {
            eprintln!("[P2P] ✗ Cannot connect to yourself! Server is running on port {port}");
            return;
        }

        // Key PeerMap chuẩn hoá host:port (không ws://) — khớp canonical key
        // sau khi peer handshake rekey, tránh duplicate connection
        let address = format!("{host}:{port}");
        if self.is_connected(&address) {
            eprintln!("[P2P] ✗ Already connected to {address}");
            return;
        }

        println!("[P2P] Connecting to ws://{address}...");
        // ponytail: handshake timeout không đặt được trực tiếp trên connect_async;
        // tự quản lý bằng timeout quanh toàn bộ connect
        let connect = tokio_tungstenite::connect_async(format!("ws://{address}"));
        match tokio::time::timeout(
            std::time::Duration::from_millis(config::WEBSOCKET_HANDSHAKE_TIMEOUT),
            connect,
        )
        .await
        {
            Ok(Ok((ws, _response))) => {
                println!("[P2P] ✓ Connected to peer: {address}");
                let node = self.node.clone();
                let peers = self.peers.clone();
                let listen_port = *self.server_port.lock().unwrap();
                tokio::spawn(connection_task(node, peers, address, ws, listen_port, true));
            }
            Ok(Err(e)) => eprintln!("[P2P] ✗ Failed to connect to {address}: {e}"),
            Err(_) => eprintln!("[P2P] ✗ Failed to connect to {address}: handshake timeout"),
        }
    }

    /// Gửi message tới mọi peer
    pub fn broadcast(&self, msg: &str) {
        broadcast(&self.peers, msg);
    }

    pub fn is_connected(&self, address: &str) -> bool {
        self.peers.lock().unwrap().contains_key(address)
    }

    pub fn get_peers(&self) -> Vec<String> {
        self.peers.lock().unwrap().keys().cloned().collect()
    }

    /// Ngắt kết nối peer theo index (1-based như JS)
    pub fn disconnect_peer(&self, index: usize) -> Result<(), String> {
        let mut peers = self.peers.lock().unwrap();
        if index == 0 || index > peers.len() {
            return Err(format!("Invalid peer index: {index}"));
        }
        // keys thu thập theo thứ tự bất kỳ — ổn cho mục đích hiển thị
        let addr = peers.keys().nth(index - 1).cloned().unwrap();
        peers.remove(&addr);
        println!("[P2P] Disconnecting peer: {addr}");
        Ok(())
    }

    pub fn disconnect_all(&self) {
        let count = self.peers.lock().unwrap().len();
        println!("[P2P] Disconnecting all {count} peer(s)...");
        self.peers.lock().unwrap().clear();
        println!("[P2P] ✓ All peers disconnected.");
    }

    /// Manual sync trigger (lệnh `sync`): broadcast handshake
    pub async fn trigger_sync(&self) -> bool {
        if self.get_peers().is_empty() {
            println!("[P2P] ⚠ No peers connected to sync with.");
            return false;
        }
        let info = self.node.lock().await.get_node_info();
        let listen_port = *self.server_port.lock().unwrap();
        self.broadcast(&messages::handshake(&info, listen_port));
        println!("[P2P] Sync request sent to all peers");
        true
    }

    pub fn close(&self) {
        self.close_server();
        self.disconnect_all();
    }
}

/// Gửi message tới một peer qua channel
pub fn send_to(peers: &PeerMap, addr: &str, msg: &str) {
    if let Some(conn) = peers.lock().unwrap().get(addr) {
        let _ = conn.tx.send(msg.to_string());
    }
}

/// Peer discovery: connect tới một peer mới tìm thấy qua handshake/PEERS.
/// Tách khỏi handler.rs vì connection_task ↔ handle_message đệ quy async —
/// spawn từ đây phá được chuỗi Send không chứng minh được.
pub fn connect_discovered(
    node: NodeHandle,
    peers: PeerMap,
    addr: String,
    listen_port: Option<u16>,
) {
    // Reserve key ngay để PEERS thứ hai (race với connection chưa kịp insert)
    // không connect đôi tới cùng peer. Placeholder tx đóng sẵn — nếu connect
    // fail thì remove key.
    {
        let mut map = peers.lock().unwrap();
        if map.contains_key(&addr) {
            return;
        }
        map.insert(addr.clone(), PeerConn::reserving());
    }
    tokio::spawn(async move {
        match tokio_tungstenite::connect_async(format!("ws://{addr}")).await {
            Ok((ws, _)) => {
                println!("[P2P] ✓ Auto-connected to discovered peer: {addr}");
                handler::connection_task(node, peers.clone(), addr, ws, listen_port, true).await;
            }
            Err(e) => {
                eprintln!("[P2P] ✗ Failed to connect to discovered peer {addr}: {e}");
                // Dọn key placeholder nếu connect fail (placeholder không thuộc
                // connection nào — xóa khi key vẫn là reserving entry)
                let mut map = peers.lock().unwrap();
                map.remove(&addr);
            }
        }
    });
}

/// Broadcast PEERS cho mọi peer hiện có (mesh discovery). Chỉ gồm addr
/// canonical (host:listenPort). Bao gồm cả chính listen port của mình —
/// node nhận thấy mình trong list thì bỏ qua.
pub fn broadcast_peers(peers: &PeerMap) {
    let addrs: Vec<String> = peers
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, c)| c.canonical)
        .map(|(addr, _)| addr.clone())
        .collect();
    if addrs.is_empty() {
        return;
    }
    broadcast(peers, &messages::peers(&addrs));
}

/// Tick định kỳ broadcast PEERS — mesh tự lành khi event-driven bị miss
/// (spawn từ cli/mod.rs như sync_watchdog).
pub async fn announce_task(peers: PeerMap) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(
        config::PEERS_ANNOUNCE_INTERVAL,
    ));
    loop {
        interval.tick().await;
        broadcast_peers(&peers);
    }
}

// Message handlers nằm ở handler.rs

/// Broadcast tới mọi peer (free fn dùng trong các handler)
pub fn broadcast(peers: &PeerMap, msg: &str) {
    for conn in peers.lock().unwrap().values() {
        let _ = conn.tx.send(msg.to_string());
    }
}

/// Relay trừ một addr (free fn dùng trong các handler)
pub fn relay_except(peers: &PeerMap, msg: &str, except: &str) -> usize {
    peers
        .lock()
        .unwrap()
        .iter()
        .filter(|(addr, _)| addr.as_str() != except)
        .filter_map(|(_, c)| c.tx.send(msg.to_string()).ok())
        .count()
}
