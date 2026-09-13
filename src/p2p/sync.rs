//! SyncManager — port từ src/p2p/SyncManager.js.
//! Quản lý đồng bộ chain giữa các node: partial/full sync, retry, timeout.
//!
//! ponytail: JS spawn setTimeout cho từng lần sync rồi gọi lại startSync (đệ quy
//! async). Ở Rust vòng đệ quy async tạo chu kỳ Send không chứng minh được, nên
//! thay bằng MỘT watchdog task kiểm tra sync stale mỗi 5s — behavior giữ nguyên.

use crate::blockchain::block::Block;
use crate::config;
use crate::node::{Node, NodeHandle};
use crate::p2p::messages;
use crate::p2p::{send_to, PeerMap};
use crate::util;

#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub is_syncing: bool,
    pub syncing_from_peer: Option<String>,
    pub last_sync_attempt: u64,
    pub retry_count: u32,
    /// Chiều cao chain của peer đang sync (để watchdog retry)
    pub peer_height: usize,
}

impl SyncState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Bắt đầu sync với một peer nếu peer có chain dài hơn.
pub async fn start_sync(node: &NodeHandle, peers: &PeerMap, peer_addr: &str, peer_height: usize) {
    let mut n = node.lock().await;
    let local_height = n.blockchain.get_latest_block().index;

    if local_height >= peer_height {
        return; // Không cần sync
    }

    let now = util::now_ms();
    if n.sync.is_syncing {
        if now.saturating_sub(n.sync.last_sync_attempt) < config::SYNC_TIMEOUT_MS {
            return; // Đang sync, chờ
        }
        println!("[SYNC] Previous sync timed out, restarting...");
    }

    n.sync.is_syncing = true;
    n.sync.syncing_from_peer = Some(peer_addr.to_string());
    n.sync.last_sync_attempt = now;
    n.sync.retry_count = 0;
    n.sync.peer_height = peer_height;

    let msg = sync_request_message(local_height, peer_height);
    println!("[SYNC] Starting sync from block {local_height} to {peer_height}...");
    drop(n);

    send_to(peers, peer_addr, &msg);
}

/// Chọn partial hay full sync request
fn sync_request_message(local_height: usize, peer_height: usize) -> String {
    let blocks_behind = peer_height - local_height;
    if blocks_behind <= config::MAX_BLOCKS_PER_REQUEST && local_height > 0 {
        println!("[SYNC] Requesting {blocks_behind} missing blocks...");
        messages::request_blocks_from(local_height + 1)
    } else {
        println!("[SYNC] Requesting full blockchain ({blocks_behind} blocks behind)...");
        messages::request_chain()
    }
}

/// Watchdog: chạy mãi, mỗi 5s kiểm tra sync đang stale (quá SYNC_TIMEOUT_MS)
/// thì đánh dấu timeout + retry, hoặc bỏ cuộc sau SYNC_MAX_RETRY lần.
/// Spawn một lần khi node khởi động.
pub async fn sync_watchdog(node: NodeHandle, peers: PeerMap) {
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
    ticker.tick().await; // tick đầu tiên là ngay lập tức — bỏ qua
    loop {
        ticker.tick().await;
        let mut n = node.lock().await;

        if !n.sync.is_syncing {
            continue;
        }
        let now = util::now_ms();
        if now.saturating_sub(n.sync.last_sync_attempt) < config::SYNC_TIMEOUT_MS {
            continue;
        }

        // Timeout
        n.sync.retry_count += 1;
        if n.sync.retry_count >= config::SYNC_MAX_RETRY {
            eprintln!("[SYNC] ✗ Max retries reached, giving up sync from this peer");
            n.sync.reset();
            continue;
        }
        println!(
            "[SYNC] ⚠ Sync timeout, retrying ({}/{})...",
            n.sync.retry_count,
            config::SYNC_MAX_RETRY
        );
        n.sync.last_sync_attempt = now;

        if let Some(peer) = n.sync.syncing_from_peer.clone() {
            let local_height = n.blockchain.get_latest_block().index;
            if local_height >= n.sync.peer_height {
                complete(&mut n);
                continue;
            }
            let msg = sync_request_message(local_height, n.sync.peer_height);
            send_to(&peers, &peer, &msg);
        }
    }
}

/// Nhận partial blocks (chỉ chấp nhận từ đúng peer đang sync).
pub async fn handle_receive_blocks(
    node: &NodeHandle,
    peers: &PeerMap,
    from_addr: &str,
    blocks: Vec<Block>,
    total_height: usize,
) {
    let mut n = node.lock().await;
    if !n.sync.is_syncing || n.sync.syncing_from_peer.as_deref() != Some(from_addr) {
        return; // Received blocks from different peer / not syncing, ignoring
    }

    println!("[SYNC] Received {} blocks", blocks.len());
    let mut added = 0;
    for block in &blocks {
        if n.receive_block(block) {
            added += 1;
        } else {
            eprintln!(
                "[SYNC] ⚠ Failed to add block #{}, stopping sync",
                block.index
            );
            break;
        }
    }
    println!("[SYNC] Added {added}/{} blocks", blocks.len());

    let current_height = n.blockchain.get_latest_block().index;
    if current_height >= total_height {
        complete(&mut n);
    } else {
        // Còn thiếu blocks, request tiếp (JS chờ timeout retry; ở đây request ngay)
        n.sync.last_sync_attempt = util::now_ms();
        let msg = messages::request_blocks_from(current_height + 1);
        drop(n);
        send_to(peers, from_addr, &msg);
    }
}

/// Nhận full chain.
pub async fn handle_receive_chain(node: &NodeHandle, chain: Vec<Block>) {
    let mut n = node.lock().await;
    if n.receive_chain(&chain) {
        complete(&mut n);
    } else {
        // Chain invalid: đánh dấu stale để watchdog retry
        n.sync.retry_count += 1;
        n.sync.last_sync_attempt = 0;
    }
}

fn complete(n: &mut Node) {
    let height = n.blockchain.get_latest_block().index;
    println!("[SYNC] ✓ Sync completed! Chain height: {height}");
    n.sync.reset();
}

/// Trạng thái sync cho lệnh `status` / `sync`
pub fn status(n: &Node) -> (bool, usize, u32) {
    (
        n.sync.is_syncing,
        n.blockchain.get_latest_block().index,
        n.sync.retry_count,
    )
}
