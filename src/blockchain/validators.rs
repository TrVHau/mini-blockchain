//! BlockValidator + TransactionValidator — port từ src/blockchain/BlockValidator.js
//! và TransactionValidator.js. Statelesss: mọi ngữ cảnh truyền qua tham số.

use std::collections::HashSet;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::{Transaction, TYPE_COINBASE};
use crate::config;
use crate::util;
use crate::wallet::BalanceTracker;

// ---- TransactionValidator ----

pub fn validate_tx_signature(tx: &Transaction) -> bool {
    if tx.signature.is_none() || tx.sender_public_key.is_none() {
        eprintln!("[TX_VALIDATOR] ✗ Transaction missing signature or public key");
        return false;
    }
    tx.is_valid()
}

pub fn validate_tx_amount(tx: &Transaction) -> bool {
    if tx.amount == 0 {
        eprintln!("[TX_VALIDATOR] ✗ Invalid transaction amount: {}", tx.amount);
        return false;
    }
    true
}

pub fn validate_tx_addresses(tx: &Transaction) -> bool {
    if !util::is_valid_address(&tx.from) {
        eprintln!("[TX_VALIDATOR] ✗ Invalid 'from' address: {}", &tx.from[..tx.from.len().min(20)]);
        return false;
    }
    if !util::is_valid_address(&tx.to) {
        eprintln!("[TX_VALIDATOR] ✗ Invalid 'to' address: {}", &tx.to[..tx.to.len().min(20)]);
        return false;
    }
    true
}

pub fn validate_tx_balance(tx: &Transaction, balances: &BalanceTracker, mempool: &[Transaction]) -> bool {
    let current_balance = balances.get_balance(&tx.from);
    let pending: i128 = mempool
        .iter()
        .filter(|m| m.from == tx.from)
        .map(|m| m.total_cost() as i128)
        .sum();
    let available = current_balance - pending;
    let required = tx.total_cost() as i128;
    if available < required {
        eprintln!(
            "[TX_VALIDATOR] ✗ Insufficient balance. Current: {}, Pending: {}, Available: {}, Required: {}",
            util::fmt_micro_i(current_balance),
            util::fmt_micro_i(pending),
            util::fmt_micro_i(available),
            util::fmt_micro(required as u64)
        );
        return false;
    }
    true
}

pub fn validate_tx_not_duplicate(tx: &Transaction, mempool: &[Transaction], spent_txids: &HashSet<String>) -> bool {
    if let Some(txid) = &tx.txid {
        if spent_txids.contains(txid) {
            eprintln!("[TX_VALIDATOR] ✗ Transaction already spent (double spend)");
            return false;
        }
        if mempool.iter().any(|m| m.txid.as_deref() == Some(txid.as_str())) {
            eprintln!("[TX_VALIDATOR] ✗ Transaction already in mempool");
            return false;
        }
    }
    let is_duplicate = mempool.iter().any(|m| {
        m.from == tx.from && m.to == tx.to && m.amount == tx.amount && m.timestamp == tx.timestamp
    });
    if is_duplicate {
        eprintln!("[TX_VALIDATOR] ✗ Duplicate transaction found in mempool");
        return false;
    }
    true
}

/// Full transaction validation
pub fn validate_transaction(tx: &Transaction, balances: &BalanceTracker, mempool: &[Transaction], spent_txids: &HashSet<String>) -> bool {
    if !validate_tx_amount(tx) {
        return false;
    }
    if !validate_tx_addresses(tx) {
        return false;
    }
    // Size limit (100KB)
    if tx.get_size() > 100_000 {
        eprintln!("[TX_VALIDATOR] ✗ Transaction too large");
        return false;
    }
    if tx.tx_type != TYPE_COINBASE && !validate_tx_signature(tx) {
        return false;
    }
    if !validate_tx_not_duplicate(tx, mempool, spent_txids) {
        return false;
    }
    if tx.tx_type != TYPE_COINBASE && !validate_tx_balance(tx, balances, mempool) {
        return false;
    }
    true
}

// ---- BlockValidator ----

#[derive(Debug, Clone, Default)]
pub struct BlockValidationOptions {
    pub difficulty: usize,
    pub expected_index: Option<usize>,
    pub expected_previous_hash: Option<String>,
    pub previous_block: Option<Block>,
    pub expected_reward: u64,
    pub max_block_size: usize,
    pub max_transactions: usize,
}

impl BlockValidationOptions {
    pub fn new(difficulty: usize) -> Self {
        Self {
            difficulty,
            max_block_size: 1_000_000,
            max_transactions: 100,
            ..Default::default()
        }
    }
}

fn validate_block_hash(block: &Block) -> bool {
    let calculated = block.calculate_hash();
    if calculated != block.hash {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} hash mismatch", block.index);
        return false;
    }
    true
}

fn validate_proof_of_work(block: &Block, difficulty: usize) -> bool {
    let target = "0".repeat(difficulty);
    if !block.hash.starts_with(&target) {
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Block #{} does not meet difficulty {}",
            block.index, difficulty
        );
        return false;
    }
    true
}

fn validate_timestamp(block: &Block, previous_block: Option<&Block>) -> bool {
    // Không được ở tương lai quá 2 giờ
    let max_future = util::now_ms() + 2 * 60 * 60 * 1000;
    if block.timestamp > max_future {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} timestamp too far in future", block.index);
        return false;
    }
    if let Some(prev) = previous_block {
        if block.timestamp < prev.timestamp {
            eprintln!(
                "[BLOCK_VALIDATOR] ✗ Block #{} timestamp before previous block",
                block.index
            );
            return false;
        }
    }
    true
}

fn validate_merkle_root(block: &Block) -> bool {
    let Some(merkle_root) = &block.merkle_root else {
        return true;
    };
    if &block.calculate_merkle_root() != merkle_root {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} merkle root mismatch", block.index);
        return false;
    }
    true
}

fn validate_coinbase(block: &Block, expected_reward: u64, total_fees: u64) -> bool {
    let Some(coinbase) = &block.coinbase_tx else {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} missing coinbase transaction", block.index);
        return false;
    };
    if coinbase.amount != expected_reward.saturating_add(total_fees) {
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Invalid coinbase amount. Expected: {}, Got: {}",
            expected_reward.saturating_add(total_fees),
            coinbase.amount
        );
        return false;
    }
    if coinbase.to.len() != 64 {
        eprintln!("[BLOCK_VALIDATOR] ✗ Invalid coinbase recipient address");
        return false;
    }
    true
}

/// Full block validation
pub fn validate_block(block: &Block, opts: &BlockValidationOptions) -> bool {
    if !validate_block_hash(block) {
        return false;
    }
    if !validate_proof_of_work(block, opts.difficulty) {
        return false;
    }
    if block.get_size() > opts.max_block_size {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} too large", block.index);
        return false;
    }
    if block.transactions.len() > opts.max_transactions {
        eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} too many transactions", block.index);
        return false;
    }
    if !validate_merkle_root(block) {
        return false;
    }
    if let Some(expected_index) = opts.expected_index {
        if block.index != expected_index {
            eprintln!(
                "[BLOCK_VALIDATOR] ✗ Block index mismatch. Expected: {expected_index}, Got: {}",
                block.index
            );
            return false;
        }
    }
    if let Some(expected_prev) = &opts.expected_previous_hash {
        if &block.previous_hash != expected_prev {
            eprintln!("[BLOCK_VALIDATOR] ✗ Block #{} previousHash mismatch", block.index);
            return false;
        }
    }
    if !validate_timestamp(block, opts.previous_block.as_ref()) {
        return false;
    }
    if block.coinbase_tx.is_some() {
        let total_fees: u64 = block.transactions.iter().map(|tx| tx.fee).sum();
        if !validate_coinbase(block, opts.expected_reward, total_fees) {
            return false;
        }
    }
    true
}

/// Validate toàn bộ chain (bỏ qua genesis, giống JS)
pub fn validate_chain(chain: &[Block], difficulty: usize) -> bool {
    if chain.is_empty() {
        eprintln!("[BLOCK_VALIDATOR] ✗ Empty chain");
        return false;
    }
    for i in 1..chain.len() {
        let opts = BlockValidationOptions {
            difficulty,
            expected_index: Some(i),
            expected_previous_hash: Some(chain[i - 1].hash.clone()),
            previous_block: Some(chain[i - 1].clone()),
            expected_reward: config::reward_at_height(i),
            max_block_size: 1_000_000,
            max_transactions: 100,
        };
        if !validate_block(&chain[i], &opts) {
            eprintln!("[BLOCK_VALIDATOR] ✗ Chain validation failed at block #{i}");
            return false;
        }
    }
    true
}
