//! Shared node state: blockchain + wallets + storage + sync state.
//! Được CLI (main thread) và các task P2P dùng chung qua Arc<tokio::sync::Mutex<Node>>.

use std::sync::Arc;

use serde::Serialize;

use crate::blockchain::block::Block;
use crate::blockchain::chain::BlockChain;
use crate::p2p::sync::SyncState;
use crate::storage::Storage;
use crate::util;
use crate::wallet::WalletManager;

/// Handle chia sẻ Node giữa CLI và các task P2P
pub type NodeHandle = Arc<tokio::sync::Mutex<Node>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub chain_height: usize,
    pub latest_block_hash: String,
    pub mempool_size: usize,
    pub timestamp: u64,
}

pub struct Node {
    pub node_id: String,
    pub blockchain: BlockChain,
    pub wallets: WalletManager,
    pub storage: Storage,
    pub sync: SyncState,
    /// automine: (miner address, interval seconds)
    pub auto_mine: Option<(String, u64)>,
    /// Handle của automine task để stop
    pub auto_mine_task: Option<tokio::task::JoinHandle<()>>,
}

impl Node {
    pub fn new(node_id: &str) -> Self {
        Self::with_base_dir(std::path::Path::new("data"), node_id)
    }

    /// Base dir tùy chỉnh (test truyền temp dir, không đụng `data/` của repo)
    pub fn with_base_dir(base: &std::path::Path, node_id: &str) -> Self {
        let storage = Storage::with_base_dir(base, node_id);
        let mut blockchain = BlockChain::new();

        // Load blockchain từ storage nếu có (giống cli.js)
        if let Some(saved) = storage.load_blockchain() {
            if saved.len() > 1 {
                let mut fresh = BlockChain::new();
                if fresh.receive_chain(&saved) {
                    blockchain = fresh;
                    println!("Loaded {} blocks from storage", saved.len());
                }
            }
        }

        Self {
            node_id: node_id.to_string(),
            blockchain,
            wallets: WalletManager::with_base_dir(base, node_id),
            storage,
            sync: SyncState::default(),
            auto_mine: None,
            auto_mine_task: None,
        }
    }

    pub fn get_node_info(&self) -> NodeInfo {
        let latest = self.blockchain.get_latest_block();
        NodeInfo {
            chain_height: latest.index,
            latest_block_hash: latest.hash.clone(),
            mempool_size: self.blockchain.mempool.len(),
            timestamp: util::now_ms(),
        }
    }

    /// Resolve địa chỉ 3 tầng như JS: wallet name -> hex address -> prefix match
    pub fn resolve_address(&self, query: &str) -> Result<String, String> {
        // 1. Local wallet
        if let Ok(addr) = self.wallets.get_address(query) {
            return Ok(addr);
        }
        // 2. Full hex address
        if util::is_valid_address(query) {
            return Ok(query.to_string());
        }
        // 3. Prefix match trong balances
        let lower = query.to_lowercase();
        let matches: Vec<String> = self
            .blockchain
            .balance_tracker
            .get_all_balances()
            .keys()
            .filter(|a| a.len() == 64 && a.to_lowercase().starts_with(&lower))
            .cloned()
            .collect();
        match matches.len() {
            1 => Ok(matches[0].clone()),
            0 => Err(format!("Wallet/address not found: \"{query}\"")),
            _ => Err(format!("Multiple addresses match \"{query}\"")),
        }
    }

    /// Nhận block từ mạng + auto-save khi thành công
    pub fn receive_block(&mut self, block: &Block) -> bool {
        if self.blockchain.receive_block(block) {
            self.storage.save_blockchain(&self.blockchain.chain);
            true
        } else {
            false
        }
    }

    /// Nhận chain từ mạng + auto-save khi thành công
    pub fn receive_chain(&mut self, chain: &[Block]) -> bool {
        if self.blockchain.receive_chain(chain) {
            self.storage.save_blockchain(&self.blockchain.chain);
            true
        } else {
            false
        }
    }

    /// Reset về genesis + save (lệnh `reset`)
    pub fn reset(&mut self) {
        self.blockchain.reset();
        self.storage.save_blockchain(&self.blockchain.chain);
    }
}

/// Mine + auto-save (JS wrapWithSave).
/// PoW chạy NGOÀI lock qua spawn_blocking — node vẫn phản hồi CLI/P2P trong lúc mine.
/// Trả về None nếu chain đổi giữa chừng (tx đã được trả về mempool).
pub async fn mine_block(node: &NodeHandle, miner_address: &str) -> Option<Block> {
    let (mut block, difficulty) = {
        let mut n = node.lock().await;
        n.blockchain.prepare_block(miner_address)
    };
    let miner = miner_address.to_string();
    let mined = tokio::task::spawn_blocking(move || {
        block.mine_block(difficulty, &miner);
        block
    })
    .await
    .expect("mine task panicked");

    let mut n = node.lock().await;
    if n.blockchain.apply_mined_block(&mined) {
        n.storage.save_blockchain(&n.blockchain.chain);
        Some(mined)
    } else {
        None
    }
}
