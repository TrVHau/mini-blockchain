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
    addr: String,
    ws: WebSocketStream<S>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    peers.lock().unwrap().insert(addr.clone(), tx);

    // Gửi handshake (JS: cả client lẫn server đều gửi khi kết nối)
    let info = node.lock().await.get_node_info();
    if sink
        .send(tokio_tungstenite::tungstenite::Message::Text(
            messages::handshake(&info),
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
    println!(
        "[P2P] Peer {addr} disconnected. Active peers: {}",
        peers.lock().unwrap().len()
    );
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
