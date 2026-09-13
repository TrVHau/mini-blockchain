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
use crate::p2p::{broadcast, broadcast_peers, relay_except, send_to, PeerConn, PeerMap};
use crate::util;

/// Kết quả xử lý 1 message trên một connection
enum Handled {
    /// Connection trùng (race 2 chiều connect nhau đồng thời) — connection này tự thoát
    Duplicate,
    /// PeerMap đã rekey sang canonical addr (peer quảng bá listenPort)
    Rekeyed(String),
    Nothing,
}

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
    outbound: bool,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Claim key PeerMap: outbound key đã canonical — chỉ nhận nếu key trống
    // hoặc còn là reservation của connect_discovered (connection thật chưa
    // claim). Key đã có connection sống = connection song song (race 2 chiều)
    // -> connection này là bản dư, tự thoát.
    // Inbound key ephemeral luôn unique -> insert thẳng.
    if outbound {
        let mut map = peers.lock().unwrap();
        match map.get(&addr) {
            None => {
                map.insert(
                    addr.clone(),
                    PeerConn {
                        tx: tx.clone(),
                        canonical: true,
                    },
                );
            }
            Some(existing) if !existing.tx.is_closed() && existing.canonical => {
                println!("[P2P] Duplicate connection to {addr}, dropping this one");
                return;
            }
            Some(_) => {
                map.insert(
                    addr.clone(),
                    PeerConn {
                        tx: tx.clone(),
                        canonical: true,
                    },
                );
            }
        }
    } else {
        peers.lock().unwrap().insert(
            addr.clone(),
            PeerConn {
                tx: tx.clone(),
                canonical: false,
            },
        );
    }

    // Gửi handshake (JS: cả client lẫn server đều gửi khi kết nối)
    let info = node.lock().await.get_node_info();
    if sink
        .send(tokio_tungstenite::tungstenite::Message::Text(
            messages::handshake(&info, listen_port),
        ))
        .await
        .is_err()
    {
        remove_own(&peers, &addr, &tx);
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
                    match handle_message(&node, &peers, &addr, &text, listen_port).await {
                        Handled::Duplicate => {
                            println!("[P2P] Duplicate connection to {addr}, dropping this one");
                            break;
                        }
                        Handled::Rekeyed(new_addr) => {
                            addr = new_addr;
                            // Event-driven mesh discovery: connection mới canonical
                            // -> quảng bá danh sách peer để các node khác tự connect
                            broadcast_peers(&peers);
                        }
                        Handled::Nothing => {}
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

    remove_own(&peers, &addr, &tx);
    println!(
        "[P2P] Peer {addr} disconnected. Active peers: {}",
        peers.lock().unwrap().len()
    );
}

/// Xóa key PeerMap chỉ khi entry còn thuộc về connection này — tránh xóa
/// entry của connection khác sau rekey/reconnect (key bị đè).
pub(super) fn remove_own(peers: &PeerMap, addr: &str, tx: &mpsc::UnboundedSender<String>) {
    let mut map = peers.lock().unwrap();
    if map.get(addr).is_some_and(|c| c.tx.same_channel(tx)) {
        map.remove(addr);
    }
}

/// Dispatch message từ peer (port MessageHandler.handle + P2P._handleSyncMessage).
/// Trả về trạng thái rekey/duplicate cho connection_task (xem `Handled`).
async fn handle_message(
    node: &NodeHandle,
    peers: &PeerMap,
    from_addr: &str,
    text: &str,
    listen_port: Option<u16>,
) -> Handled {
    let Ok((msg_type, data)) = messages::parse(text) else {
        eprintln!("[P2P] ✗ Error processing message from {from_addr}");
        return Handled::Nothing;
    };

    // Rekey khi peer quảng bá listen port qua HANDSHAKE:
    // ephemeral addr -> host:listenPort (địa chỉ danh tính, discovery connect lại được)
    let mut handled = Handled::Nothing;
    let from_addr_owned: String;
    let from_addr: &str = if msg_type == "HANDSHAKE" {
        match data["listenPort"].as_u64() {
            Some(port) => {
                let host = from_addr
                    .rsplit_once(':')
                    .map(|(h, _)| h)
                    .unwrap_or("127.0.0.1");
                let new_addr = format!("{host}:{port}");
                let mut map = peers.lock().unwrap();
                if new_addr != from_addr && map.contains_key(&new_addr) {
                    // Connection khác đã giữ key canonical (race 2 chiều connect
                    // đồng thời, hoặc node đó đã connect tới mình trước) —
                    // connection inbound này là bản dư
                    handled = Handled::Duplicate;
                    from_addr_owned = new_addr;
                    from_addr_owned.as_str()
                } else {
                    // Dọn key cũ trỏ connection này (ephemeral addr inbound),
                    // insert lại key canonical
                    let conn = map.remove(from_addr);
                    map.insert(
                        new_addr.clone(),
                        PeerConn {
                            tx: conn.expect("key from_addr phải thuộc connection này").tx,
                            canonical: true,
                        },
                    );
                    handled = Handled::Rekeyed(new_addr.clone());
                    from_addr_owned = new_addr;
                    from_addr_owned.as_str()
                }
            }
            None => from_addr,
        }
    } else {
        from_addr
    };

    match msg_type.as_str() {
        messages::message_type::HANDSHAKE => {
            let peer_height = data["chainHeight"].as_u64().unwrap_or(0) as usize;
            println!("[P2P] Handshake received from peer (height: {peer_height})");
            // Chỉ announce addr canonical — ephemeral addr không connect lại được
            let peer_addrs: Vec<String> = {
                let map = peers.lock().unwrap();
                map.iter()
                    .filter(|(a, c)| c.canonical && a.as_str() != from_addr)
                    .map(|(a, _)| a.clone())
                    .collect()
            };
            let my_info = node.lock().await.get_node_info();
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
            // Peer discovery: connect tới các peer của peer
            if let Some(addrs) = data["peers"].as_array() {
                let addrs: Vec<String> = addrs
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                discover_peers(node, peers, &addrs, from_addr, listen_port);
            }
        }
        messages::message_type::PEERS => {
            // Mesh động: danh sách peer của một node khác -> connect addr chưa biết
            if let Some(addrs) = data["peers"].as_array() {
                let addrs: Vec<String> = addrs
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                discover_peers(node, peers, &addrs, "", listen_port);
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
                return handled;
            };
            let total_height = data["totalHeight"].as_u64().unwrap_or(0) as usize;
            sync::handle_receive_blocks(node, peers, from_addr, blocks, total_height).await;
        }
        messages::message_type::NEW_BLOCK => {
            let Ok(block) = serde_json::from_value::<Block>(data["block"].clone()) else {
                eprintln!("[P2P] ✗ Invalid NEW_BLOCK message: missing block data");
                return handled;
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
                return handled;
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
                return handled;
            };
            handle_transaction(node, peers, from_addr, tx).await;
        }
        other => println!("[P2P] Unhandled message type: {other}"),
    }
    handled
}

/// Peer discovery dùng chung cho HANDSHAKE_ACK và PEERS: connect tới các
/// addr chưa biết (còn slot, không phải chính mình / addr gửi).
fn discover_peers(
    node: &NodeHandle,
    peers: &PeerMap,
    addrs: &[String],
    from_addr: &str,
    listen_port: Option<u16>,
) {
    let known: Vec<String> = peers.lock().unwrap().keys().cloned().collect();
    if known.len() >= config::MAX_PEERS {
        return;
    }
    // Filter self: mọi biến thể host localhost trên listen port của mình
    // (peer có thể quảng bá "localhost:3001" hay "127.0.0.1:3001" cho cùng node)
    let is_self = |addr: &str| match (addr.rsplit_once(':'), listen_port) {
        (Some((host, port)), Some(my_port)) => {
            port.parse::<u16>() == Ok(my_port) && util::is_localhost(host)
        }
        _ => false,
    };
    for addr in addrs {
        if is_self(addr) || addr == from_addr {
            continue;
        }
        if known.iter().any(|k| k == addr) {
            continue;
        }
        println!("[P2P] Discovered new peer: {addr}");
        crate::p2p::connect_discovered(node.clone(), peers.clone(), addr.clone(), listen_port);
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
