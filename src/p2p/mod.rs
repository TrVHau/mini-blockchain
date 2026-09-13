//! P2P — port từ src/p2p/P2P.js + PeerManager.js + MessageHandler.js + BlockHandler.js.
//! Mỗi kết nối WebSocket = 1 tokio task; outgoing qua unbounded channel.

pub mod messages;
pub mod sync;

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::config;
use crate::node::NodeHandle;
use crate::util;

/// addr -> channel gửi message ra peer đó
pub type PeerMap = Arc<StdMutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

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
        *self.server_port.lock().unwrap() = Some(port);

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
                        tokio::spawn(connection_task(node.clone(), peers.clone(), addr, ws));
                    }
                    Err(e) => eprintln!("[P2P] ✗ WebSocket handshake failed: {e}"),
                }
            }
        });
        *self.server_task.lock().unwrap() = Some(handle);
        println!("[P2P] ✓ P2P server running on port {port}");
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

        let address = format!("ws://{host}:{port}");
        if self.is_connected(&address) {
            eprintln!("[P2P] ✗ Already connected to {address}");
            return;
        }

        println!("[P2P] Connecting to {address}...");
        // ponytail: handshake timeout không đặt được trực tiếp trên connect_async;
        // tự quản lý bằng timeout quanh toàn bộ connect
        let connect = tokio_tungstenite::connect_async(address.clone());
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
                tokio::spawn(connection_task(node, peers, address, ws));
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
        self.broadcast(&messages::handshake(&info));
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
    if let Some(tx) = peers.lock().unwrap().get(addr) {
        let _ = tx.send(msg.to_string());
    }
}

/// Task cho một kết nối WebSocket (cả inbound lẫn outbound):
/// pump outgoing channel + xử lý incoming messages.
/// Generic vì inbound là WebSocketStream<TcpStream>,
/// outbound là WebSocketStream<MaybeTlsStream<TcpStream>>.
async fn connection_task<S>(node: NodeHandle, peers: PeerMap, addr: String, ws: WebSocketStream<S>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    peers.lock().unwrap().insert(addr.clone(), tx);

    // Gửi handshake (JS: cả client lẫn server đều gửi khi kết nối)
    let info = node.lock().await.get_node_info();
    if sink.send(Message::Text(messages::handshake(&info))).await.is_err() {
        peers.lock().unwrap().remove(&addr);
        return;
    }

    loop {
        tokio::select! {
            out = rx.recv() => match out {
                Some(msg) => {
                    if sink.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    handle_message(&node, &peers, &addr, &text).await;
                }
                Some(Ok(_)) => {} // ping/pong/binary: bỏ qua
                Some(Err(e)) => {
                    eprintln!("[P2P] ✗ Socket error from {addr}: {e}");
                    break;
                }
                None => break,
            },
        }
    }

    peers.lock().unwrap().remove(&addr);
    println!("[P2P] Peer {addr} disconnected. Active peers: {}", peers.lock().unwrap().len());
}

/// Dispatch message từ peer (port MessageHandler.handle + P2P._handleSyncMessage)
async fn handle_message(node: &NodeHandle, peers: &PeerMap, from_addr: &str, text: &str) {
    let Ok((msg_type, data)) = messages::parse(text) else {
        eprintln!("[P2P] ✗ Error processing message from {from_addr}");
        return;
    };

    match msg_type.as_str() {
        messages::message_type::HANDSHAKE => {
            let peer_height = data["chainHeight"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Handshake received from peer (height: {peer_height})");
            let my_info = node.lock().await.get_node_info();
            send_to(peers, from_addr, &messages::handshake_ack(&my_info));
            sync::start_sync(node, peers, from_addr, peer_height).await;
        }
        messages::message_type::HANDSHAKE_ACK => {
            let peer_height = data["chainHeight"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Handshake ACK received (peer height: {peer_height})");
            sync::start_sync(node, peers, from_addr, peer_height).await;
        }
        messages::message_type::REQUEST_BLOCKS_FROM => {
            let from_index = data["fromIndex"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Peer requesting blocks from index {from_index}");
            let (blocks, total_height) = {
                let n = node.lock().await;
                let chain = &n.blockchain.chain;
                let end = (from_index + config::MAX_BLOCKS_PER_REQUEST).min(chain.len());
                (chain[from_index.min(chain.len())..end].to_vec(), chain.len() - 1)
            };
            send_to(peers, from_addr, &messages::receive_blocks(&blocks, from_index, total_height));
            println!("[P2P] Sent {} blocks to peer", blocks.len());
        }
        messages::message_type::RECEIVE_BLOCKS => {
            let Ok(blocks) = serde_json::from_value::<Vec<Block>>(data["blocks"].clone()) else {
                eprintln!("[P2P] ✗ Invalid RECEIVE_BLOCKS message");
                return;
            };
            let total_height = data["totalHeight"].as_u64().unwrap_or(0) as usize;
            sync::handle_receive_blocks(node, peers, from_addr, blocks, total_height).await;
        }
        messages::message_type::NEW_BLOCK => {
            let Ok(block) = serde_json::from_value::<Block>(data["block"].clone()) else {
                eprintln!("[P2P] ✗ Invalid NEW_BLOCK message: missing block data");
                return;
            };
            handle_new_block(node, peers, from_addr, block).await;
        }
        messages::message_type::REQUEST_CHAIN => {
            let chain = node.lock().await.blockchain.chain.clone();
            send_to(peers, from_addr, &messages::receive_chain(&chain));
            println!("[P2P] Sent blockchain to requesting peer");
        }
        messages::message_type::RECEIVE_CHAIN => {
            let Ok(chain) = serde_json::from_value::<Vec<Block>>(data["chain"].clone()) else {
                eprintln!("[P2P] ✗ Invalid RECEIVE_CHAIN message: missing chain data");
                return;
            };
            sync::handle_receive_chain(node, chain).await;
        }
        messages::message_type::REQUEST_LATEST => {
            let latest = node.lock().await.blockchain.get_latest_block().clone();
            send_to(peers, from_addr, &messages::new_block(&latest));
        }
        messages::message_type::TRANSACTION => {
            let Ok(tx) = serde_json::from_value::<Transaction>(data["transaction"].clone()) else {
                eprintln!("[P2P] ✗ Invalid TRANSACTION message: missing transaction data");
                return;
            };
            handle_transaction(node, peers, from_addr, tx).await;
        }
        other => println!("[P2P] Unhandled message type: {other}"),
    }
}

/// Port BlockHandler.handleNewBlock
async fn handle_new_block(node: &NodeHandle, peers: &PeerMap, from_addr: &str, block: Block) {
    let (accepted, latest_index) = {
        let mut n = node.lock().await;
        let latest_index = n.blockchain.get_latest_block().index;
        if block.index == latest_index + 1 {
            let accepted = n.receive_block(&block);
            if accepted {
                println!("[P2P] New block #{} synchronized from network", block.index);
            }
            (accepted, latest_index)
        } else if block.index > latest_index + 1 {
            println!(
                "[P2P] Blockchain behind (local: {latest_index}, received: {}). Requesting full chain...",
                block.index
            );
            (false, latest_index)
        } else {
            // Block cũ hoặc đã có
            (true, latest_index)
        }
    };

    if block.index == latest_index + 1 {
        if accepted {
            let msg = messages::new_block(&block);
            let relayed = relay_except(peers, &msg, from_addr);
            if relayed > 0 {
                println!("[P2P] Block #{} relayed to {relayed} peer(s)", block.index);
            }
        } else {
            // Block bị từ chối -> request full chain
            broadcast(peers, &messages::request_chain());
        }
    } else if block.index > latest_index + 1 {
        broadcast(peers, &messages::request_chain());
    }
}

/// Port MessageHandler.handleTransaction
async fn handle_transaction(node: &NodeHandle, peers: &PeerMap, from_addr: &str, tx: Transaction) {
    let accepted = {
        let mut n = node.lock().await;
        match n.blockchain.add_transaction(&tx) {
            Ok(_) => {
                println!("[P2P] Transaction added to mempool from peer");
                true
            }
            Err(e) => {
                println!("[P2P] Transaction from peer rejected: {e}");
                false
            }
        }
    };
    if accepted {
        let msg = messages::transaction(&tx);
        let relayed = relay_except(peers, &msg, from_addr);
        if relayed > 0 {
            println!("[P2P] Transaction relayed to {relayed} peer(s)");
        }
    }
}

/// Broadcast tới mọi peer (free fn dùng trong các handler)
pub fn broadcast(peers: &PeerMap, msg: &str) {
    for tx in peers.lock().unwrap().values() {
        let _ = tx.send(msg.to_string());
    }
}

/// Relay trừ một addr (free fn dùng trong các handler)
pub fn relay_except(peers: &PeerMap, msg: &str, except: &str) -> usize {
    peers
        .lock()
        .unwrap()
        .iter()
        .filter(|(addr, _)| addr.as_str() != except)
        .filter_map(|(_, tx)| tx.send(msg.to_string()).ok())
        .count()
}
