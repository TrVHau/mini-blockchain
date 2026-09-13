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
        let storage = Storage::new(node_id);
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
            wallets: WalletManager::new(node_id),
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

    /// Mine + auto-save (JS wrapWithSave)
    pub fn mine_block(&mut self, miner_address: &str) -> Block {
        let block = self.blockchain.mine_block(miner_address);
        self.storage.save_blockchain(&self.blockchain.chain);
        block
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
