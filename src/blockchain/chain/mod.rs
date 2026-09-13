//! BlockChain — port từ src/blockchain/BlockChain.js.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::blockchain::validators;
use crate::config;
use crate::util;
use crate::wallet::BalanceTracker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInfo {
    pub transaction: Transaction,
    pub block_index: usize,
    pub confirmations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub history_type: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub block_index: usize,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total_blocks: usize,
    pub total_transactions: usize,
    pub total_coins: i128,
    pub difficulty: usize,
    pub mempool_size: usize,
    pub avg_block_time: u64,
    pub latest_block_hash: String,
    pub spent_tx_count: usize,
}

pub struct BlockChain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
    pub mempool: Vec<Transaction>,
    pub balance_tracker: BalanceTracker,
    /// Track spent transactions (double spend protection)
    pub spent_txids: HashSet<String>,
}

impl BlockChain {
    pub fn new() -> Self {
        Self::with_difficulty(config::DEFAULT_DIFFICULTY)
    }

    pub fn with_difficulty(difficulty: usize) -> Self {
        Self {
            chain: vec![Block::genesis()],
            difficulty,
            mempool: Vec::new(),
            balance_tracker: BalanceTracker::default(),
            spent_txids: HashSet::new(),
        }
    }

    /// Reset về genesis (lệnh `reset` của CLI)
    pub fn reset(&mut self) {
        self.chain = vec![Block::genesis()];
        self.mempool.clear();
        self.spent_txids.clear();
        self.balance_tracker.update_balance(&self.chain);
    }

    pub fn get_latest_block(&self) -> &Block {
        self.chain.last().expect("chain luôn có ít nhất genesis")
    }

    pub fn get_block(&self, index: usize) -> Option<&Block> {
        self.chain.get(index)
    }

    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.chain.iter().find(|b| b.hash == hash)
    }

    /// Tìm transaction theo txid
    pub fn get_transaction(&self, txid: &str) -> Option<TxInfo> {
        for block in &self.chain {
            if let Some(tx) = block
                .transactions
                .iter()
                .find(|t| t.txid.as_deref() == Some(txid))
            {
                return Some(TxInfo {
                    transaction: tx.clone(),
                    block_index: block.index,
                    confirmations: self.get_confirmations(block.index),
                });
            }
        }
        None
    }

    pub fn get_confirmations(&self, block_index: usize) -> usize {
        self.chain
            .len()
            .saturating_sub(1)
            .saturating_sub(block_index)
    }

    pub fn is_confirmed(&self, txid: &str) -> bool {
        match self.get_transaction(txid) {
            Some(info) => info.confirmations >= config::CONFIRMATIONS_REQUIRED,
            None => false,
        }
    }

    /// Sync difficulty cho block tiếp theo theo retarget rule — gọi chung
    /// expected_difficulty với validator để miner và validator không lệch nhau
    /// (cửa sổ đo từ block index - interval, không phải index - interval + 1).
    pub fn adjust_difficulty(&mut self) {
        if let Some(d) = validators::expected_difficulty(&self.chain, self.chain.len()) {
            if d != self.difficulty {
                println!("[BLOCKCHAIN] Difficulty adjusted to {d}");
                self.difficulty = d;
            }
        }
    }

    /// Nhận block từ mạng — validate rồi thêm vào chain.
    /// Difficulty phải khớp retarget rule suy từ chain (tăng HAY giảm đều
    /// chỉ được xảy ra tại boundary) — mirror adjust_difficulty của miner.
    pub fn receive_block(&mut self, block: &Block) -> bool {
        let latest = self.get_latest_block().clone();
        let declared = validators::declared_difficulty(block);
        let difficulty_ok = match validators::expected_difficulty(&self.chain, block.index) {
            Some(expected) => declared == expected,
            // Block #1: chưa có mốc retarget — chỉ chặn ngoài [MIN, MAX]
            None => (config::MIN_DIFFICULTY..=config::MAX_DIFFICULTY).contains(&declared),
        };
        if !difficulty_ok {
            eprintln!(
                "[BLOCKCHAIN] ✗ Block #{} difficulty {declared} không khớp retarget rule",
                block.index
            );
            return false;
        }
        let opts = validators::BlockValidationOptions {
            difficulty: declared,
            expected_index: Some(latest.index + 1),
            expected_previous_hash: Some(latest.hash.clone()),
            previous_block: Some(latest),
            expected_reward: config::reward_at_height(self.chain.len()),
            max_block_size: config::MAX_BLOCK_SIZE,
            max_transactions: config::MAX_TRANSACTIONS_PER_BLOCK,
        };
        if !validators::validate_block(block, &opts) {
            eprintln!("[BLOCKCHAIN] ✗ Block #{} validation failed", block.index);
            return false;
        }
        // Trust boundary: validate từng tx trong block (chữ ký, double-spend,
        // số dư cộng dồn) — block từ peer không được tin.
        if !validators::validate_block_transactions(block, &self.balance_tracker, &self.spent_txids)
        {
            eprintln!(
                "[BLOCKCHAIN] ✗ Block #{} contains invalid transactions",
                block.index
            );
            return false;
        }

        let block = block.clone();
        self.chain.push(block.clone());
        self.track_spent(&block);
        // Incremental balance update (chỉ block mới)
        self.balance_tracker.process_block(&block);
        // Xóa các transactions đã confirm khỏi mempool
        self.remove_confirmed_transactions(&block);
        // Sync difficulty mining với tip mới (giờ biết window retarget thật)
        self.adjust_difficulty();
        println!(
            "[BLOCKCHAIN] ✓ Block #{} accepted and added to chain",
            block.index
        );
        true
    }

    /// Đánh dấu txid của các transaction trong block là đã spent
    /// (double-spend protection — mọi đường thêm block vào chain đều qua đây).
    fn track_spent(&mut self, block: &Block) {
        for tx in &block.transactions {
            if let Some(txid) = &tx.txid {
                self.spent_txids.insert(txid.clone());
            }
        }
    }

    /// Xóa các transactions đã confirm trong block khỏi mempool
    fn remove_confirmed_transactions(&mut self, block: &Block) {
        if block.transactions.is_empty() {
            return;
        }
        let confirmed = &block.transactions;
        self.mempool.retain(|mempool_tx| {
            if let Some(txid) = &mempool_tx.txid {
                if confirmed.iter().any(|c| c.txid.as_ref() == Some(txid)) {
                    return false;
                }
            }
            // Fallback content matching (giống JS)
            !confirmed.iter().any(|c| {
                c.from == mempool_tx.from
                    && c.to == mempool_tx.to
                    && c.amount == mempool_tx.amount
                    && c.timestamp == mempool_tx.timestamp
            })
        });
    }

    pub fn is_chain_valid(&self) -> bool {
        validators::validate_chain(&self.chain, self.difficulty)
    }

    /// Nhận chain từ mạng — chỉ thay thế nếu dài hơn строго
    pub fn receive_chain(&mut self, new_chain: &[Block]) -> bool {
        if !validators::validate_chain(new_chain, self.difficulty) {
            eprintln!("[BLOCKCHAIN] ✗ Received chain is invalid");
            return false;
        }
        if new_chain.len() <= self.chain.len() {
            println!(
                "[BLOCKCHAIN] Received chain is not longer than current chain ({} <= {})",
                new_chain.len(),
                self.chain.len()
            );
            return false;
        }
        println!(
            "[BLOCKCHAIN] Replacing current chain ({} blocks) with new chain ({} blocks)",
            self.chain.len(),
            new_chain.len()
        );
        self.chain = new_chain.to_vec();
        self.balance_tracker.update_balance(&self.chain);
        self.spent_txids.clear();
        for block in new_chain {
            self.track_spent(block);
        }
        // Sync difficulty cục bộ theo retarget rule của chain mới (để mine
        // block sau tiếp nối đúng difficulty của mạng)
        self.adjust_difficulty();
        // Reset mempool khi nhận chain mới vì các tx cũ có thể không còn valid
        self.mempool.clear();
        true
    }

    /// Thêm transaction (validate) vào mempool
    pub fn add_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        if !validators::validate_transaction(
            tx,
            &self.balance_tracker,
            &self.mempool,
            &self.spent_txids,
        ) {
            return Err("Transaction validation failed".to_string());
        }
        if self.mempool.len() >= config::MAX_TRANSACTIONS_PER_BLOCK * 2 {
            return Err("Mempool is full. Please wait or increase fee.".to_string());
        }
        self.mempool.push(tx.clone());
        Ok(())
    }

    /// Mine block mới: chọn tx từ mempool (fee cao trước), PoW, cập nhật state.
    /// Gồm cả 3 bước: prepare → mine → apply (dùng khi giữ lock suốt cũng ổn).
    #[allow(dead_code)] // dùng trong test; luồng runtime đi qua node::mine_block (spawn_blocking)
    pub fn mine_block(&mut self, miner_address: &str) -> Block {
        let (mut block, difficulty) = self.prepare_block(miner_address);
        block.mine_block(difficulty, miner_address);
        // Luồng đồng bộ: không ai đổi chain giữa chừng nên apply luôn thành công
        assert!(
            self.apply_mined_block(&block),
            "apply_mined_block thất bại ở luồng đồng bộ — logic prepare/apply lệch nhau"
        );
        block
    }

    /// Bước 1 (chạy dưới lock): snapshot latest block + chọn tx từ mempool
    /// (ưu tiên fee cao, giới hạn số lượng + size), xóa tx đã chọn khỏi mempool.
    /// Trả về block CHƯA mine + difficulty hiện tại.
    pub fn prepare_block(&mut self, miner_address: &str) -> (Block, usize) {
        let pre_block = self.get_latest_block().clone();
        let mut new_block = Block::new(
            pre_block.index + 1,
            None,
            &pre_block.hash,
            Some(miner_address.to_string()),
        );

        let mut sorted_mempool = self.mempool.clone();
        sorted_mempool.sort_by_key(|tx| std::cmp::Reverse(tx.fee));

        let mut selected: Vec<Transaction> = Vec::new();
        let mut current_size = 0usize;
        for tx in sorted_mempool {
            if selected.len() >= config::MAX_TRANSACTIONS_PER_BLOCK {
                break;
            }
            let tx_size = tx.get_size();
            if current_size + tx_size > config::MAX_BLOCK_SIZE {
                continue;
            }
            current_size += tx_size;
            selected.push(tx);
        }
        new_block.transactions = selected;

        // Xóa các tx đã chọn khỏi mempool (tx không txid không thể vào mempool
        // — validate_transaction chặn từ cửa)
        let selected_keys: HashSet<String> = new_block
            .transactions
            .iter()
            .map(|tx| tx.txid.clone().unwrap_or_default())
            .collect();
        self.mempool
            .retain(|tx| !selected_keys.contains(tx.txid.as_deref().unwrap_or("")));

        (new_block, self.difficulty)
    }

    /// Bước 3 (chạy dưới lock): áp block đã mine vào chain.
    /// Nếu chain đã đổi giữa lúc mine (peer thêm block khác) → từ chối,
    /// trả các transactions của block về mempool.
    pub fn apply_mined_block(&mut self, block: &Block) -> bool {
        let latest = self.get_latest_block();
        if latest.hash != block.previous_hash || latest.index + 1 != block.index {
            println!(
                "[BLOCKCHAIN] ⚠ Chain changed while mining (local tip #{}), discarding mined block #{}",
                latest.index, block.index
            );
            // Tx của block bị bỏ được trả về mempool (chúng đã bị remove ở prepare).
            // Tx không có txid không bao giờ vào mempool qua add_transaction nên không bị trùng.
            self.mempool.extend(block.transactions.iter().cloned());
            return false;
        }

        self.track_spent(block);

        self.chain.push(block.clone());
        self.balance_tracker.process_block(block);
        self.adjust_difficulty();
        true
    }

    pub fn get_balance(&self, address: &str) -> i128 {
        self.balance_tracker.get_balance(address)
    }

    pub fn get_transactions_history(&self, address: &str) -> Vec<HistoryEntry> {
        let mut history = Vec::new();
        for block in &self.chain {
            if let Some(coinbase) = &block.coinbase_tx {
                if coinbase.to == address {
                    history.push(HistoryEntry {
                        history_type: "MINING_REWARD".to_string(),
                        from: util::SYSTEM_SENDER.to_string(),
                        to: coinbase.to.clone(),
                        amount: coinbase.amount,
                        fee: 0,
                        timestamp: block.timestamp,
                        block_index: block.index,
                        hash: util::prefix(&block.hash, 16),
                    });
                }
            }
            for tx in &block.transactions {
                if tx.from == address || tx.to == address {
                    history.push(HistoryEntry {
                        history_type: if tx.from == address {
                            "SENT"
                        } else {
                            "RECEIVED"
                        }
                        .to_string(),
                        from: tx.from.clone(),
                        to: tx.to.clone(),
                        amount: tx.amount,
                        fee: tx.fee,
                        timestamp: block.timestamp,
                        block_index: block.index,
                        hash: util::prefix(&block.hash, 16),
                    });
                }
            }
        }
        history
    }

    /// Block reward hiện tại (cho block tiếp theo, có halving)
    pub fn get_block_reward(&self) -> u64 {
        config::reward_at_height(self.chain.len())
    }

    pub fn get_stats(&self) -> Stats {
        let total_transactions: usize = self.chain.iter().map(|b| b.transactions.len()).sum();
        let total_coins: i128 = self.balance_tracker.get_all_balances().values().sum();

        // Average block time (last 10 blocks)
        let recent = &self.chain[self.chain.len().saturating_sub(10)..];
        let mut avg_block_time = 0u64;
        if recent.len() > 1 {
            let diffs: Vec<u64> = recent
                .windows(2)
                .map(|w| w[1].timestamp.saturating_sub(w[0].timestamp))
                .collect();
            if !diffs.is_empty() {
                avg_block_time = diffs.iter().sum::<u64>() / diffs.len() as u64 / 1000;
                // seconds
            }
        }

        Stats {
            total_blocks: self.chain.len(),
            total_transactions,
            total_coins,
            difficulty: self.difficulty,
            mempool_size: self.mempool.len(),
            avg_block_time,
            latest_block_hash: self.get_latest_block().hash.clone(),
            spent_tx_count: self.spent_txids.len(),
        }
    }

    /// Transactions trong mempool, sắp xếp theo fee
    pub fn get_pending_transactions(&self) -> Vec<&Transaction> {
        let mut pending: Vec<&Transaction> = self.mempool.iter().collect();
        pending.sort_by_key(|tx| std::cmp::Reverse(tx.fee));
        pending
    }

    /// Ước tính fee để transaction được xử lý nhanh (trung bình mempool + 10%)
    pub fn estimate_fee(&self) -> u64 {
        if self.mempool.is_empty() {
            return 0;
        }
        let n = self.mempool.len() as u64;
        let sum: u64 = self.mempool.iter().map(|tx| tx.fee).sum();
        // ceil(sum/n * 1.1) = ceil(sum*11 / (n*10))
        sum.saturating_mul(11).div_ceil(n * 10)
    }
}

impl Default for BlockChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
