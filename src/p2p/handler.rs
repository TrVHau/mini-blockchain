//! P2P message handlers — tách từ mod.rs (port MessageHandler + BlockHandler).
//! connection_task + dispatch + handle_new_block + handle_transaction.

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::config;
use crate::node::NodeHandle;
use crate::p2p::messages;
use crate::p2p::sync;
use crate::p2p::{broadcast, relay_except, send_to, PeerMap};

/// Task cho một kết nối WebSocket (cả inbound lẫn outbound):
/// pump outgoing channel + xử lý incoming messages.
/// Generic vì inbound là WebSocketStream<TcpStream>,
/// outbound là WebSocketStream<MaybeTlsStream<TcpStream>>.
pub(super) async fn connection_task<S>(
    node: NodeHandle,
    peers: PeerMap,
    mut addr: String,
    ws: WebSocketStream<S>,
    listen_port: Option<u16>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Key PeerMap ban đầu là ephemeral addr; sau khi peer gửi handshake
    // (quảng bá listenPort) handle_message rekey sang host:listenPort
    // và trả về canonical addr — connection này cập nhật theo.
    peers.lock().unwrap().insert(addr.clone(), tx);

    // Gửi handshake (JS: cả client lẫn server đều gửi khi kết nối)
    let info = node.lock().await.get_node_info();
    if sink
        .send(tokio_tungstenite::tungstenite::Message::Text(
            messages::handshake(&info, listen_port),
        ))
        .await
        .is_err()
    {
        peers.lock().unwrap().remove(&addr);
        return;
    }

    loop {
        tokio::select! {
            out = rx.recv() => match out {
                Some(msg) => {
                    if sink.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
            incoming = stream.next() => match incoming {
                Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                    // handle_message có thể rekey PeerMap (peer quảng bá listenPort)
                    // -> cập nhật addr canonical cho các message sau trên connection này
                    if let Some(new_addr) = handle_message(&node, &peers, &addr, &text).await {
                        addr = new_addr;
                    }
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
    println!(
        "[P2P] Peer {addr} disconnected. Active peers: {}",
        peers.lock().unwrap().len()
    );
}

/// Dispatch message từ peer (port MessageHandler.handle + P2P._handleSyncMessage).
/// Trả về Some(canonical_addr) nếu handshake của peer yêu cầu rekey PeerMap
/// (peer quảng bá listenPort — key ổn định thay vì ephemeral port).
async fn handle_message(
    node: &NodeHandle,
    peers: &PeerMap,
    from_addr: &str,
    text: &str,
) -> Option<String> {
    let Ok((msg_type, data)) = messages::parse(text) else {
        eprintln!("[P2P] ✗ Error processing message from {from_addr}");
        return None;
    };

    // Rekey khi peer quảng bá listen port qua HANDSHAKE:
    // ephemeral addr -> host:listenPort (địa chỉ danh tính, discovery connect lại được)
    let canonical: Option<String>;
    let from_addr_owned: String;
    let from_addr: &str = if msg_type == "HANDSHAKE" {
        match data["listenPort"].as_u64() {
            Some(port) => {
                let host = from_addr
                    .rsplit_once(':')
                    .map(|(h, _)| h)
                    .unwrap_or("127.0.0.1");
                let new_addr = format!("{host}:{port}");
                if new_addr != from_addr {
                    let mut map = peers.lock().unwrap();
                    // Dọn key cũ trỏ connection này: ephemeral addr (inbound)
                    // hoặc ws://host:port (outbound), rồi rekey canonical
                    let tx = map
                        .remove(from_addr)
                        .or_else(|| map.remove(&format!("ws://{new_addr}")));
                    if let Some(tx) = tx {
                        map.insert(new_addr.clone(), tx);
                    }
                }
                canonical = Some(new_addr.clone());
                from_addr_owned = new_addr;
                from_addr_owned.as_str()
            }
            None => {
                canonical = None;
                from_addr
            }
        }
    } else {
        canonical = None;
        from_addr
    };

    match msg_type.as_str() {
        messages::message_type::HANDSHAKE => {
            let peer_height = data["chainHeight"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Handshake received from peer (height: {peer_height})");
            let peer_addrs: Vec<String> = peers.lock().unwrap().keys().cloned().collect();
            let my_info = node.lock().await.get_node_info();
            // Trừ chính node vừa gửi handshake để không connect ngược lại nó
            let peer_addrs: Vec<String> =
                peer_addrs.into_iter().filter(|a| a != from_addr).collect();
            send_to(
                peers,
                from_addr,
                &messages::handshake_ack(&my_info, &peer_addrs),
            );
            sync::start_sync(node, peers, from_addr, peer_height).await;
        }
        messages::message_type::HANDSHAKE_ACK => {
            let peer_height = data["chainHeight"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Handshake ACK received (peer height: {peer_height})");
            sync::start_sync(node, peers, from_addr, peer_height).await;
            // Peer discovery: connect tới các peer của peer (không có sẵn + còn slot)
            if let Some(addrs) = data["peers"].as_array() {
                let known: Vec<String> = peers.lock().unwrap().keys().cloned().collect();
                if known.len() < config::MAX_PEERS {
                    for addr in addrs.iter().filter_map(|v| v.as_str()) {
                        if addr == from_addr || known.iter().any(|k| k == addr) {
                            continue;
                        }
                        println!("[P2P] Discovered new peer via handshake: {addr}");
                        crate::p2p::connect_discovered(
                            node.clone(),
                            peers.clone(),
                            addr.to_string(),
                        );
                    }
                }
            }
        }
        messages::message_type::REQUEST_BLOCKS_FROM => {
            let from_index = data["fromIndex"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Peer requesting blocks from index {from_index}");
            let (blocks, total_height) = {
                let n = node.lock().await;
                let chain = &n.blockchain.chain;
                let end = (from_index + config::MAX_BLOCKS_PER_REQUEST).min(chain.len());
                (
                    chain[from_index.min(chain.len())..end].to_vec(),
                    chain.len() - 1,
                )
            };
            send_to(
                peers,
                from_addr,
                &messages::receive_blocks(&blocks, from_index, total_height),
            );
            println!("[P2P] Sent {} blocks to peer", blocks.len());
        }
        messages::message_type::RECEIVE_BLOCKS => {
            let Ok(blocks) = serde_json::from_value::<Vec<Block>>(data["blocks"].clone()) else {
                eprintln!("[P2P] ✗ Invalid RECEIVE_BLOCKS message");
                return canonical;
            };
            let total_height = data["totalHeight"].as_u64().unwrap_or(0) as usize;
            sync::handle_receive_blocks(node, peers, from_addr, blocks, total_height).await;
        }
        messages::message_type::NEW_BLOCK => {
            let Ok(block) = serde_json::from_value::<Block>(data["block"].clone()) else {
                eprintln!("[P2P] ✗ Invalid NEW_BLOCK message: missing block data");
                return canonical;
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
                return canonical;
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
                return canonical;
            };
            handle_transaction(node, peers, from_addr, tx).await;
        }
        other => println!("[P2P] Unhandled message type: {other}"),
    }
    canonical
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
        match n.add_transaction(&tx) {
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
