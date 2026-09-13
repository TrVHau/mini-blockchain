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

    #[allow(dead_code)] // API port từ JS, chưa có caller
    pub fn get_block(&self, index: usize) -> Option<&Block> {
        self.chain.get(index)
    }

    #[allow(dead_code)] // API port từ JS, chưa có caller
    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.chain.iter().find(|b| b.hash == hash)
    }

    /// Tìm transaction theo txid
    pub fn get_transaction(&self, txid: &str) -> Option<TxInfo> {
        for block in &self.chain {
            if let Some(tx) = block.transactions.iter().find(|t| t.txid.as_deref() == Some(txid)) {
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
        self.chain.len().saturating_sub(1).saturating_sub(block_index)
    }

    pub fn is_confirmed(&self, txid: &str) -> bool {
        match self.get_transaction(txid) {
            Some(info) => info.confirmations >= config::CONFIRMATIONS_REQUIRED,
            None => false,
        }
    }

    /// Điều chỉnh difficulty dựa trên thời gian mining (mỗi DIFFICULTY_ADJUSTMENT_INTERVAL blocks)
    pub fn adjust_difficulty(&mut self) {
        let interval = config::DIFFICULTY_ADJUSTMENT_INTERVAL;
        let latest = self.get_latest_block();
        if latest.index % interval != 0 || latest.index == 0 {
            return;
        }
        let prev_adjustment = &self.chain[self.chain.len() - interval];
        let time_expected = (interval as u64) * config::TARGET_BLOCK_TIME;
        let time_taken = latest.timestamp.saturating_sub(prev_adjustment.timestamp);

        if time_taken < time_expected / 2 {
            self.difficulty = (self.difficulty + 1).min(config::MAX_DIFFICULTY);
            println!("[BLOCKCHAIN] Difficulty increased to {}", self.difficulty);
        } else if time_taken > time_expected * 2 {
            self.difficulty = self.difficulty.saturating_sub(1).max(config::MIN_DIFFICULTY);
            println!("[BLOCKCHAIN] Difficulty decreased to {}", self.difficulty);
        }
    }

    /// Nhận block từ mạng — validate rồi thêm vào chain
    pub fn receive_block(&mut self, block: &Block) -> bool {
        let latest = self.get_latest_block().clone();
        let opts = validators::BlockValidationOptions {
            difficulty: self.difficulty,
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

        let block = block.clone();
        self.chain.push(block.clone());
        // Incremental balance update (chỉ block mới)
        self.balance_tracker.process_block(&block);
        // Xóa các transactions đã confirm khỏi mempool
        self.remove_confirmed_transactions(&block);
        println!("[BLOCKCHAIN] ✓ Block #{} accepted and added to chain", block.index);
        true
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
        // Reset mempool khi nhận chain mới vì các tx cũ có thể không còn valid
        self.mempool.clear();
        true
    }

    /// Thêm transaction (validate) vào mempool
    pub fn add_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        if !validators::validate_transaction(tx, &self.balance_tracker, &self.mempool, &self.spent_txids) {
            return Err("Transaction validation failed".to_string());
        }
        if self.mempool.len() >= config::MAX_TRANSACTIONS_PER_BLOCK * 2 {
            return Err("Mempool is full. Please wait or increase fee.".to_string());
        }
        self.mempool.push(tx.clone());
        Ok(())
    }

    /// Mine block mới: chọn tx từ mempool (fee cao trước), PoW, cập nhật state
    pub fn mine_block(&mut self, miner_address: &str) -> Block {
        let pre_block = self.get_latest_block().clone();
        let mut new_block = Block::new(pre_block.index + 1, None, &pre_block.hash, Some(miner_address.to_string()));

        // Chọn transactions từ mempool (ưu tiên fee cao, giới hạn số lượng + size)
        let mut sorted_mempool = self.mempool.clone();
        sorted_mempool.sort_by(|a, b| b.fee.cmp(&a.fee));

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

        // Xóa các tx đã chọn khỏi mempool
        let selected_keys: HashSet<String> = new_block
            .transactions
            .iter()
            .map(|tx| tx.txid.clone().unwrap_or_default())
            .collect();
        self.mempool
            .retain(|tx| !selected_keys.contains(tx.txid.as_deref().unwrap_or("")) || tx.txid.is_none());

        // Mine block
        new_block.mine_block(self.difficulty, miner_address);

        // Track spent transactions
        for tx in &new_block.transactions {
            if let Some(txid) = &tx.txid {
                self.spent_txids.insert(txid.clone());
            }
        }

        self.chain.push(new_block.clone());
        self.balance_tracker.process_block(&new_block);
        self.adjust_difficulty();

        new_block
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
                        hash: format!("{}...", &block.hash[..16]),
                    });
                }
            }
            for tx in &block.transactions {
                if tx.from == address || tx.to == address {
                    history.push(HistoryEntry {
                        history_type: if tx.from == address { "SENT" } else { "RECEIVED" }.to_string(),
                        from: tx.from.clone(),
                        to: tx.to.clone(),
                        amount: tx.amount,
                        fee: tx.fee,
                        timestamp: block.timestamp,
                        block_index: block.index,
                        hash: format!("{}...", &block.hash[..16]),
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
            let diffs: Vec<u64> = recent.windows(2).map(|w| w[1].timestamp.saturating_sub(w[0].timestamp)).collect();
            if !diffs.is_empty() {
                avg_block_time = diffs.iter().sum::<u64>() / diffs.len() as u64 / 1000; // seconds
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
        pending.sort_by(|a, b| b.fee.cmp(&a.fee));
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
        (sum.saturating_mul(11) + n * 10 - 1) / (n * 10)
    }
}

impl Default for BlockChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto;

    fn miner() -> (String, String, String) {
        let (sk, pk) = crypto::generate_keypair();
        let addr = crypto::address_from_public_hex(&pk).unwrap();
        (sk, pk, addr)
    }

    fn signed_tx(from_sk: &str, from_pk: &str, from_addr: &str, to: &str, amount: u64, fee: u64) -> Transaction {
        let mut tx = Transaction::new(from_addr, to, amount, fee);
        tx.sign(from_sk, from_pk).unwrap();
        tx
    }

    #[test]
    fn genesis_is_deterministic_and_valid() {
        let g1 = Block::genesis();
        let g2 = Block::genesis();
        assert_eq!(g1.hash, g2.hash);
        assert_eq!(g1.index, 0);
        assert_eq!(g1.timestamp, config::GENESIS_TIMESTAMP);
        let bc = BlockChain::new();
        assert!(bc.is_chain_valid());
    }

    #[test]
    fn mine_and_validate_chain() {
        let (_, _, miner_addr) = miner();
        let mut bc = BlockChain::with_difficulty(2);
        let block = bc.mine_block(&miner_addr);
        assert!(block.hash.starts_with("00"));
        assert_eq!(bc.chain.len(), 2);
        assert_eq!(bc.get_balance(&miner_addr), 16_000_000);
        assert!(bc.is_chain_valid());

        // Coinbase amount = reward + fees (fee = 0)
        assert_eq!(block.coinbase_tx.as_ref().unwrap().amount, 16_000_000);
    }

    #[test]
    fn send_coins_and_double_spend_protection() {
        let (sk_a, pk_a, addr_a) = miner();
        let (_, _, addr_b) = miner();
        let mut bc = BlockChain::with_difficulty(2);
        bc.mine_block(&addr_a); // A có 16 coins

        // Gửi 5 coins A -> B
        let tx = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 5_000_000, 0);
        bc.add_transaction(&tx).unwrap();
        assert_eq!(bc.mempool.len(), 1);
        bc.mine_block(&addr_b);
        assert_eq!(bc.get_balance(&addr_a), 11_000_000);
        assert_eq!(bc.get_balance(&addr_b), 16_000_000 + 5_000_000);

        // Double spend: cùng txid đã nằm trong block -> bị từ chối
        assert!(bc.add_transaction(&tx).is_err());

        // Vượt số dư
        let bad = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 999_000_000, 0);
        assert!(bc.add_transaction(&bad).is_err());
    }

    #[test]
    fn pending_balance_blocks_mempool_overspend() {
        let (sk_a, pk_a, addr_a) = miner();
        let (_, _, addr_b) = miner();
        let mut bc = BlockChain::with_difficulty(2);
        bc.mine_block(&addr_a);

        let tx1 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 10_000_000, 0);
        bc.add_transaction(&tx1).unwrap();
        // Đã pending 10 coins, chỉ còn 6 -> tx 7 coins phải fail
        let tx2 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 7_000_000, 0);
        assert!(bc.add_transaction(&tx2).is_err());
    }

    #[test]
    fn receive_chain_only_if_longer() {
        let (_, _, miner_addr) = miner();
        let mut bc1 = BlockChain::with_difficulty(2);
        bc1.mine_block(&miner_addr);
        bc1.mine_block(&miner_addr);

        let mut bc2 = BlockChain::with_difficulty(2);
        assert!(bc2.receive_chain(&bc1.chain));
        assert_eq!(bc2.chain.len(), 3);
        assert_eq!(bc2.get_balance(&miner_addr), 32_000_000);

        // Chain ngắn hơn hoặc bằng -> từ chối
        assert!(!bc2.receive_chain(&bc1.chain[..2]));

        // Chain hỏng (sửa hash) -> từ chối
        let mut tampered = bc1.chain.clone();
        tampered[1].nonce += 1;
        assert!(!bc2.receive_chain(&tampered));
    }

    #[test]
    fn receive_block_validates_linkage() {
        let (_, _, miner_addr) = miner();
        let mut bc1 = BlockChain::with_difficulty(2);
        bc1.mine_block(&miner_addr);
        let block1 = bc1.chain[1].clone();

        let mut bc2 = BlockChain::with_difficulty(2);
        assert!(bc2.receive_block(&block1));
        assert_eq!(bc2.chain.len(), 2);

        // Block trùng index -> fail
        assert!(!bc2.receive_block(&block1));
    }

    #[test]
    fn estimate_fee_math() {
        let mut bc = BlockChain::new();
        assert_eq!(bc.estimate_fee(), 0);
        bc.mempool.push(Transaction::new(&"a".repeat(64), &"b".repeat(64), 1, 1_000_000));
        bc.mempool.push(Transaction::new(&"a".repeat(64), &"b".repeat(64), 1, 2_000_000));
        // avg = 1.5, *1.1 = 1.65 -> ceil = 1_650_000
        assert_eq!(bc.estimate_fee(), 1_650_000);
    }

    #[test]
    fn halving_via_mining() {
        let (_, _, miner_addr) = miner();
        let mut bc = BlockChain::with_difficulty(1);
        // Mine đến block 50 (index 50) — reward phải halving còn 8 coins
        while bc.chain.len() <= config::HALVING_INTERVAL {
            bc.mine_block(&miner_addr);
        }
        let last = bc.get_latest_block();
        assert_eq!(last.coinbase_tx.as_ref().unwrap().amount, 8_000_000);
        assert_eq!(bc.get_block_reward(), 8_000_000);
    }
}
