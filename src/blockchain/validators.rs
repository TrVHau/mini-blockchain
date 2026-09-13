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
        eprintln!(
            "[TX_VALIDATOR] ✗ Invalid 'from' address: {}",
            &tx.from[..tx.from.len().min(20)]
        );
        return false;
    }
    if !util::is_valid_address(&tx.to) {
        eprintln!(
            "[TX_VALIDATOR] ✗ Invalid 'to' address: {}",
            &tx.to[..tx.to.len().min(20)]
        );
        return false;
    }
    true
}

pub fn validate_tx_balance(
    tx: &Transaction,
    balances: &BalanceTracker,
    mempool: &[Transaction],
) -> bool {
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

pub fn validate_tx_not_duplicate(
    tx: &Transaction,
    mempool: &[Transaction],
    spent_txids: &HashSet<String>,
) -> bool {
    if let Some(txid) = &tx.txid {
        if spent_txids.contains(txid) {
            eprintln!("[TX_VALIDATOR] ✗ Transaction already spent (double spend)");
            return false;
        }
        if mempool
            .iter()
            .any(|m| m.txid.as_deref() == Some(txid.as_str()))
        {
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
pub fn validate_transaction(
    tx: &Transaction,
    balances: &BalanceTracker,
    mempool: &[Transaction],
    spent_txids: &HashSet<String>,
) -> bool {
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

/// Validate các transactions TRONG block nhận từ mạng: chữ ký + không double-spend
/// (txid chưa từng nằm trong chain) + số dư cộng dồn theo thứ tự trong block.
/// Đây là trust boundary — block từ peer không được tin khi add vào chain.
pub fn validate_block_transactions(
    block: &Block,
    balances: &BalanceTracker,
    spent_txids: &HashSet<String>,
) -> bool {
    // Balance tạm: số dư cộng dồn trong block (tx sau thấy tx trước đã trừ)
    let mut rolling = balances.clone();
    for tx in &block.transactions {
        if !validate_tx_signature(tx) {
            return false;
        }
        let Some(txid) = &tx.txid else {
            eprintln!("[TX_VALIDATOR] ✗ Transaction in block missing txid");
            return false;
        };
        if spent_txids.contains(txid) {
            eprintln!("[TX_VALIDATOR] ✗ Transaction in block already spent (double spend)");
            return false;
        }
        let available = rolling.get_balance(&tx.from);
        if available < tx.total_cost() as i128 {
            eprintln!(
                "[TX_VALIDATOR] ✗ Transaction in block #{} exceeds balance. Available: {}, Required: {}",
                block.index,
                util::fmt_micro_i(available),
                util::fmt_micro(tx.total_cost())
            );
            return false;
        }
        rolling.debit(&tx.from, tx.total_cost());
        rolling.credit(&tx.to, tx.amount);
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
    #[allow(dead_code)] // API port từ JS, chưa có caller
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

/// Difficulty thực tế của một block đã mine = số leading-zeros của hash.
/// Dùng khi validate block/chain từ mạng: node nhận không thể tin `self.difficulty`
/// cục bộ (peer có thể đã adjust khác) — suy từ chính proof-of-work của block.
pub fn pow_difficulty(block: &Block) -> usize {
    block.hash.chars().take_while(|c| *c == '0').count()
}

fn validate_timestamp(block: &Block, previous_block: Option<&Block>) -> bool {
    // Không được ở tương lai quá 2 giờ
    let max_future = util::now_ms() + 2 * 60 * 60 * 1000;
    if block.timestamp > max_future {
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Block #{} timestamp too far in future",
            block.index
        );
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
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Block #{} merkle root mismatch",
            block.index
        );
        return false;
    }
    true
}

fn validate_coinbase(block: &Block, expected_reward: u64, total_fees: u64) -> bool {
    let Some(coinbase) = &block.coinbase_tx else {
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Block #{} missing coinbase transaction",
            block.index
        );
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
        eprintln!(
            "[BLOCK_VALIDATOR] ✗ Block #{} too many transactions",
            block.index
        );
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
            eprintln!(
                "[BLOCK_VALIDATOR] ✗ Block #{} previousHash mismatch",
                block.index
            );
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

/// Validate toàn bộ chain (bỏ qua genesis, giống JS).
/// Difficulty kỳ vọng cho block i = leading-zeros của block i-1 (miner i-1
/// đã adjust theo cùng thuật toán) — stateless, không tin difficulty cục bộ
/// của node nhận. Block có PoW yếu hơn block trước vẫn bị từ chối.
pub fn validate_chain(chain: &[Block], _local_difficulty: usize) -> bool {
    if chain.is_empty() {
        eprintln!("[BLOCK_VALIDATOR] ✗ Empty chain");
        return false;
    }
    // Balance + spent txids tích lũy theo chain — mỗi block thấy hiệu ứng
    // của các block trước nó (trust boundary: không tin tx trong chain nhận từ mạng)
    let mut rolling_balances = BalanceTracker::default();
    let mut rolling_spent: HashSet<String> = HashSet::new();
    for i in 1..chain.len() {
        // ponytail: lấy min giữa difficulty của block trước và của chính block i
        // — block i phải đạt ít nhất mức tip trước đó; cao hơn thì chấp nhận
        // (peer đã adjust tăng là hợp lệ).
        let expected = pow_difficulty(&chain[i - 1]);
        let actual = pow_difficulty(&chain[i]);
        if actual < expected {
            eprintln!(
                "[BLOCK_VALIDATOR] ✗ Block #{} PoW difficulty dropped ({} < {})",
                i, actual, expected
            );
            return false;
        }
        let opts = BlockValidationOptions {
            difficulty: expected,
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
        if !validate_block_transactions(&chain[i], &rolling_balances, &rolling_spent) {
            eprintln!(
                "[BLOCK_VALIDATOR] ✗ Chain validation failed at block #{i} (invalid transactions)"
            );
            return false;
        }
        // Cập nhật rolling state cho block tiếp theo
        if let Some(coinbase) = &chain[i].coinbase_tx {
            rolling_balances.credit(&coinbase.to, coinbase.amount);
        }
        for tx in &chain[i].transactions {
            if let Some(txid) = &tx.txid {
                rolling_spent.insert(txid.clone());
            }
            rolling_balances.debit(&tx.from, tx.total_cost());
            rolling_balances.credit(&tx.to, tx.amount);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn miner() -> (String, String) {
        let (sk, pk) = crate::crypto::generate_keypair();
        (sk, pk)
    }

    /// Block #1 đã mine hợp lệ + opts khớp để validate nó
    fn valid_block() -> (Block, BlockValidationOptions) {
        let (_, pk) = miner();
        let addr = crate::crypto::address_from_public_hex(&pk).unwrap();
        let mut bc = crate::blockchain::chain::BlockChain::with_difficulty(2);
        let block = bc.mine_block(&addr);
        let opts = BlockValidationOptions {
            difficulty: 2,
            expected_index: Some(1),
            expected_previous_hash: Some(bc.chain[0].hash.clone()),
            previous_block: Some(bc.chain[0].clone()),
            expected_reward: config::reward_at_height(1),
            max_block_size: config::MAX_BLOCK_SIZE,
            max_transactions: config::MAX_TRANSACTIONS_PER_BLOCK,
        };
        (block, opts)
    }

    /// Tamper xong thì recompute hash để đi qua được check hash, chạm đúng check đích
    fn rehash(block: &mut Block) {
        block.hash = block.calculate_hash();
    }

    #[test]
    fn valid_block_passes() {
        let (block, opts) = valid_block();
        assert!(validate_block(&block, &opts));
    }

    #[test]
    fn wrong_index_fails() {
        let (mut block, mut opts) = valid_block();
        block.index = 2;
        rehash(&mut block);
        opts.expected_index = Some(1);
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn wrong_previous_hash_fails() {
        let (mut block, opts) = valid_block();
        block.previous_hash = "ff".repeat(32);
        rehash(&mut block);
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn insufficient_difficulty_fails() {
        let (block, mut opts) = valid_block();
        opts.difficulty = block.hash.len().min(6); // hash không thể có 6+ số 0 đầu
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn tampered_merkle_root_fails() {
        let (mut block, opts) = valid_block();
        block.merkle_root = Some("ff".repeat(32));
        rehash(&mut block);
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn tampered_coinbase_amount_fails() {
        let (mut block, opts) = valid_block();
        block.coinbase_tx.as_mut().unwrap().amount += 1;
        rehash(&mut block);
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn block_too_large_fails() {
        let (block, mut opts) = valid_block();
        opts.max_block_size = 1;
        assert!(!validate_block(&block, &opts));
    }

    #[test]
    fn timestamp_in_future_fails() {
        let (mut block, opts) = valid_block();
        block.timestamp = util::now_ms() + 3 * 60 * 60 * 1000; // +3h
        rehash(&mut block);
        assert!(!validate_block(&block, &opts));
    }
}
