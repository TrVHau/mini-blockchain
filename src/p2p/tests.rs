//! Integration test P2P: 2 node thật trên localhost — ephemeral port + temp dir riêng
//! (không đụng `data/` của repo, không đổi current_dir).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::P2P;
use crate::blockchain::transaction::Transaction;
use crate::node::{self, Node, NodeHandle};
use crate::p2p::messages;

static NODE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Node với temp dir riêng, port/dir không đụng test khác chạy song song
fn temp_node(name: &str) -> NodeHandle {
    let id = NODE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mbc-p2p-test-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Arc::new(tokio::sync::Mutex::new(Node::with_base_dir(&dir, name)))
}

/// Poll cho đến khi f() == true hoặc hết timeout
async fn wait_for<F, Fut>(timeout: Duration, mut f: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if f().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    f().await
}

async fn start_pair() -> (NodeHandle, Arc<P2P>, NodeHandle, Arc<P2P>) {
    let a = temp_node("a");
    let b = temp_node("b");
    let p2p_a = P2P::new(a.clone());
    let p2p_b = P2P::new(b.clone());
    p2p_a.start_server(0).await;
    p2p_b.start_server(0).await;
    let port_a = p2p_a
        .server_port
        .lock()
        .unwrap()
        .expect("server A đã start");
    p2p_b.connect_to_peer("127.0.0.1", port_a).await;
    (a, p2p_a, b, p2p_b)
}

async fn chain_len(node: &NodeHandle) -> usize {
    node.lock().await.blockchain.chain.len()
}

#[tokio::test]
async fn block_relay_between_nodes() {
    let (a, p2p_a, b, _p2p_b) = start_pair().await;

    let miner = "aa".repeat(32);
    let block = node::mine_block(&a, &miner).await.expect("mine thành công");
    p2p_a.broadcast(&messages::new_block(&block));

    let ok = wait_for(Duration::from_secs(10), || async {
        chain_len(&b).await >= 2
    })
    .await;
    assert!(ok, "B phải nhận được block của A qua NEW_BLOCK relay");
    let height_b = { b.lock().await.blockchain.get_latest_block().index };
    assert_eq!(height_b, block.index);
}

#[tokio::test]
async fn late_joiner_syncs_full_chain() {
    // A mine 3 blocks TRƯỚC khi B kết nối
    let a = temp_node("a");
    let miner = "ab".repeat(32);
    for _ in 0..3 {
        node::mine_block(&a, &miner).await.expect("mine thành công");
    }

    let p2p_a = P2P::new(a.clone());
    p2p_a.start_server(0).await;
    let port_a = p2p_a
        .server_port
        .lock()
        .unwrap()
        .expect("server A đã start");

    let b = temp_node("b");
    let p2p_b = P2P::new(b.clone());
    p2p_b.start_server(0).await;
    p2p_b.connect_to_peer("127.0.0.1", port_a).await;

    let ok = wait_for(Duration::from_secs(10), || async {
        chain_len(&b).await == 4
    })
    .await;
    assert!(ok, "B tham gia muộn phải sync đủ chain của A");
    assert_eq!(chain_len(&a).await, chain_len(&b).await);
}

#[tokio::test]
async fn transaction_relay_then_replay_rejected() {
    let (a, p2p_a, b, _p2p_b) = start_pair().await;

    let addr_a = {
        let mut n = a.lock().await;
        n.wallets.create_wallet("alice").expect("tạo wallet")
    };
    let (sk, pk) = {
        let n = a.lock().await;
        (
            n.wallets.get_private_key("alice").unwrap(),
            n.wallets.get_public_key_hex("alice").unwrap(),
        )
    };

    // Block 1:alice có tiền
    let block1 = node::mine_block(&a, &addr_a)
        .await
        .expect("mine thành công");
    p2p_a.broadcast(&messages::new_block(&block1));
    let ok = wait_for(Duration::from_secs(10), || async {
        chain_len(&b).await >= 2
    })
    .await;
    assert!(ok, "B phải nhận block 1");

    // Tx alice -> ai đó, broadcast
    let to = "cd".repeat(32);
    let mut tx = Transaction::new(&addr_a, &to, 1_000_000, 0);
    tx.sign(&sk, &pk).expect("ký tx");
    {
        let mut n = a.lock().await;
        n.blockchain.add_transaction(&tx).expect("tx hợp lệ");
    }
    p2p_a.broadcast(&messages::transaction(&tx));
    let txid = tx.txid.clone().unwrap();
    let ok = wait_for(Duration::from_secs(10), || async {
        let n = b.lock().await;
        n.blockchain
            .mempool
            .iter()
            .any(|m| m.txid.as_deref() == Some(txid.as_str()))
    })
    .await;
    assert!(ok, "B phải nhận tx vào mempool qua TRANSACTION relay");

    // Block 2 chứa tx
    let block2 = node::mine_block(&a, &addr_a)
        .await
        .expect("mine thành công");
    p2p_a.broadcast(&messages::new_block(&block2));
    let ok = wait_for(Duration::from_secs(10), || async {
        chain_len(&b).await >= 3
    })
    .await;
    assert!(ok, "B phải nhận block 2");

    // Regression spent_txids: replay tx đã confirm trên B -> phải bị từ chối
    let mut n = b.lock().await;
    assert!(
        n.blockchain.add_transaction(&tx).is_err(),
        "replay tx đã confirm qua block từ mạng phải bị từ chối (spent_txids)"
    );
}
