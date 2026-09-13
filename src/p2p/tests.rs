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
async fn mutual_connect_converges_to_one_connection() {
    // A và B connect tới nhau ĐỒNG THỜI (race 2 chiều). Trước đây cả hai bên
    // cùng drop inbound -> cả 2 connection chết -> discovery connect lại ->
    // race lại -> vòng lặp kết nối vô hạn. Giờ phải hội tụ về đúng 1
    // connection và ỔN ĐỊNH (không flap).
    let a = temp_node("a");
    let b = temp_node("b");
    let p2p_a = P2P::new(a.clone());
    let p2p_b = P2P::new(b.clone());
    p2p_a.start_server(0).await;
    p2p_b.start_server(0).await;
    let port_a = p2p_a.server_port.lock().unwrap().expect("server A");
    let port_b = p2p_b.server_port.lock().unwrap().expect("server B");

    // B connect A bằng "localhost" — biến thể host khác "127.0.0.1" của cùng
    // một addr, trước đây tạo 2 key khác nhau phá duplicate detection
    let (ra, rb) = (
        p2p_a.connect_to_peer("localhost", port_b),
        p2p_b.connect_to_peer("127.0.0.1", port_a),
    );
    tokio::join!(ra, rb);

    // Chờ race giải quyết rồi kiểm tra ổn định: đúng 1 peer, giữ nguyên sau 3s
    let ok = wait_for(Duration::from_secs(10), || async {
        p2p_a.get_peers().len() == 1 && p2p_b.get_peers().len() == 1
    })
    .await;
    assert!(ok, "cả hai node phải có đúng 1 peer sau race");
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(p2p_a.get_peers().len(), 1, "không được flap sau khi hội tụ");
    assert_eq!(p2p_b.get_peers().len(), 1, "không được flap sau khi hội tụ");
    // Key phải là dạng chuẩn hoá, không phải "localhost:..."
    assert!(p2p_a.get_peers()[0].starts_with("127.0.0.1:"));
}

#[test]
fn canonical_addr_normalizes_localhost_variants() {
    assert_eq!(super::canonical_addr("localhost:3001"), "127.0.0.1:3001");
    assert_eq!(super::canonical_addr("LOCALHOST:3001"), "127.0.0.1:3001");
    assert_eq!(
        super::canonical_addr("example.com:3001"),
        "example.com:3001"
    );
    assert_eq!(super::canonical_addr("bad"), "bad");
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

#[tokio::test]
async fn disconnect_closes_connection() {
    // B connect vào A; B disconnect — phải đóng WebSocket THẬT (task của B
    // thoát, A thấy peer biến mất). Trước đây task vẫn sống sau disconnect:
    // message từ A vẫn được xử lý, A vẫn thấy connection.
    let (_a, p2p_a, _b, p2p_b) = start_pair().await;
    let ok = wait_for(Duration::from_secs(10), || async {
        p2p_a.get_peers().len() == 1 && p2p_b.get_peers().len() == 1
    })
    .await;
    assert!(ok, "A và B phải có 1 peer sau khi connect");

    p2p_b.disconnect_peer(1).expect("disconnect peer index 1");

    let ok = wait_for(Duration::from_secs(10), || async {
        p2p_a.get_peers().is_empty()
    })
    .await;
    assert!(ok, "A phải thấy connection đóng sau khi B disconnect");
}

#[tokio::test]
async fn discovered_peer_connect_failure_cleans_reservation() {
    // Connect tới port không có listener -> fail -> placeholder key phải
    // được dọn để discovery sau này còn thử lại được
    let a = temp_node("a");
    let p2p_a = P2P::new(a.clone());
    super::connect_discovered(
        a.clone(),
        p2p_a.peers.clone(),
        "127.0.0.1:1".to_string(),
        None,
    );
    let ok = wait_for(Duration::from_secs(10), || async {
        p2p_a.peers.lock().unwrap().is_empty()
    })
    .await;
    assert!(ok, "placeholder phải được dọn sau khi connect fail");
}

#[tokio::test]
async fn mesh_updates_when_new_node_joins() {
    // A-B, A-C handshake xong (B-C đã biết nhau qua ACK). D join sau qua A —
    // B và C phải tự connect D nhờ PEERS broadcast event-driven từ A
    // (không cần tick 60s của announce_task).
    let a = temp_node("a");
    let b = temp_node("b");
    let c = temp_node("c");
    let d = temp_node("d");
    let p2p_a = P2P::new(a.clone());
    let p2p_b = P2P::new(b.clone());
    let p2p_c = P2P::new(c.clone());
    let p2p_d = P2P::new(d.clone());

    p2p_a.start_server(0).await;
    p2p_b.start_server(0).await;
    p2p_c.start_server(0).await;
    p2p_d.start_server(0).await;
    let port_a = p2p_a.server_port.lock().unwrap().expect("server A");
    let port_d = p2p_d.server_port.lock().unwrap().expect("server D");

    // A-B, A-C; chờ mesh A-B-C hình thành (B-C biết nhau qua handshake ACK)
    p2p_b.connect_to_peer("127.0.0.1", port_a).await;
    p2p_c.connect_to_peer("127.0.0.1", port_a).await;
    let ok = wait_for(Duration::from_secs(15), || async {
        p2p_a.get_peers().len() == 2 && p2p_b.get_peers().len() == 2 && p2p_c.get_peers().len() == 2
    })
    .await;
    assert!(ok, "mesh A-B-C phải hình thành (mỗi node 2 peer)");

    // D join sau — connect A như một node mới
    p2p_d.connect_to_peer("127.0.0.1", port_a).await;

    // B và C phải tự connect D trong 15s nhờ PEERS broadcast từ A
    let ok = wait_for(Duration::from_secs(15), || async {
        p2p_b.is_connected(&format!("127.0.0.1:{port_d}"))
            && p2p_c.is_connected(&format!("127.0.0.1:{port_d}"))
    })
    .await;
    assert!(ok, "B và C phải tự connect D qua PEERS broadcast");
}

#[tokio::test]
async fn peer_discovery_via_handshake() {
    // A-B và A-C kết nối; handshake ACK của A liệt kê peers -> B tự connect C
    let a = temp_node("a");
    let b = temp_node("b");
    let c = temp_node("c");
    let p2p_a = P2P::new(a.clone());
    let p2p_b = P2P::new(b.clone());
    let p2p_c = P2P::new(c.clone());

    p2p_a.start_server(0).await;
    p2p_b.start_server(0).await;
    p2p_c.start_server(0).await;
    let port_a = p2p_a.server_port.lock().unwrap().expect("server A");
    let port_b = p2p_b.server_port.lock().unwrap().expect("server B");
    let port_c = p2p_c.server_port.lock().unwrap().expect("server C");

    // B và C đều connect tới A
    p2p_b.connect_to_peer("127.0.0.1", port_a).await;
    p2p_c.connect_to_peer("127.0.0.1", port_a).await;
    let ok = wait_for(Duration::from_secs(10), || async {
        p2p_a.get_peers().len() == 2
    })
    .await;
    assert!(ok, "A phải có 2 peer (B và C)");

    // Peer discovery: B và C phải tự tìm thấy nhau qua handshake ACK của A
    let ok = wait_for(Duration::from_secs(15), || async {
        p2p_b.is_connected(&format!("127.0.0.1:{port_c}"))
            || p2p_c.is_connected(&format!("127.0.0.1:{port_b}"))
    })
    .await;
    assert!(ok, "B và C phải tự kết nối với nhau qua peer discovery");

    // Block mined ở B relay đến C (qua A hoặc trực tiếp — mesh đã hình thành)
    let miner = "ff".repeat(32);
    let block = node::mine_block(&b, &miner).await.expect("mine");
    p2p_b.broadcast(&messages::new_block(&block));
    let ok = wait_for(Duration::from_secs(10), || async {
        chain_len(&c).await >= 2
    })
    .await;
    assert!(ok, "C phải nhận block của B qua mesh");
}
