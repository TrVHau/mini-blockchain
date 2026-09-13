//! Block — port từ src/blockchain/Block.js.

use serde::{Deserialize, Serialize};

use crate::blockchain::transaction::CoinbaseTransaction;
use crate::blockchain::transaction::Transaction;
use crate::config;
use crate::crypto;
use crate::merkle;
use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub index: usize,
    pub data: Option<String>,
    pub previous_hash: String,
    pub timestamp: u64,
    pub nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub miner_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coinbase_tx: Option<CoinbaseTransaction>,
    #[serde(default)]
    pub transactions: Vec<Transaction>,
    #[serde(default)]
    pub total_fees: u64,
    /// Merkle root của transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_root: Option<String>,
    pub hash: String,
}

impl Block {
    pub fn new(index: usize, data: Option<String>, previous_hash: &str, miner_address: Option<String>) -> Self {
        let mut block = Self {
            index,
            data,
            previous_hash: previous_hash.to_string(),
            timestamp: util::now_ms(),
            nonce: 0,
            miner_address,
            coinbase_tx: None,
            transactions: Vec::new(),
            total_fees: 0,
            merkle_root: None,
            hash: String::new(),
        };
        block.hash = block.calculate_hash();
        block
    }

    /// Genesis block cố định
    pub fn genesis() -> Self {
        let mut genesis = Self::new(
            0,
            Some(config::GENESIS_DATA.to_string()),
            config::GENESIS_PREVIOUS_HASH,
            None,
        );
        genesis.timestamp = config::GENESIS_TIMESTAMP;
        genesis.merkle_root = Some(genesis.calculate_merkle_root());
        genesis.hash = genesis.calculate_hash();
        genesis
    }

    /// Danh sách leaf hashes cho merkle tree (coinbase leaf đứng đầu nếu có)
    pub fn merkle_leaves(&self) -> Vec<String> {
        let mut tx_hashes: Vec<String> = self.transactions.iter().map(|tx| tx.txid.clone().unwrap_or_else(|| tx.calculate_hash())).collect();
        if let Some(coinbase) = &self.coinbase_tx {
            let coinbase_hash = merkle::hash(&format!("{}|{}|{}", coinbase.to, coinbase.amount, self.index));
            tx_hashes.insert(0, coinbase_hash);
        }
        tx_hashes
    }

    /// Tính Merkle Root từ transactions
    pub fn calculate_merkle_root(&self) -> String {
        merkle::calculate_root(&self.merkle_leaves())
    }

    /// Hash của block header:
    /// sha256("{index}|{previousHash}|{timestamp}|{nonce}|{merkleRoot||''}")
    pub fn calculate_hash(&self) -> String {
        crypto::sha256_hex(
            format!(
                "{}|{}|{}|{}|{}",
                self.index,
                self.previous_hash,
                self.timestamp,
                self.nonce,
                self.merkle_root.as_deref().unwrap_or("")
            )
            .as_bytes(),
        )
    }

    /// Mine block với Proof of Work
    pub fn mine_block(&mut self, difficulty: usize, miner_address: &str) {
        self.total_fees = self.transactions.iter().map(|tx| tx.fee).sum();
        self.coinbase_tx = Some(CoinbaseTransaction::new(miner_address, self.index, self.total_fees));
        // Tính merkle root sau khi có tất cả transactions
        self.merkle_root = Some(self.calculate_merkle_root());

        let target = "0".repeat(difficulty);
        loop {
            self.nonce += 1;
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
        }
    }

    /// Verify một transaction có trong block không (dùng Merkle Proof).
    /// ponytail: JS tính proof không tính coinbase leaf (bug — proof không khớp
    /// root khi có coinbase); ở đây dùng chung leaf list với root để proof chạy đúng.
    #[allow(dead_code)] // API học tập (JS cũng chỉ expose, không gọi từ CLI)
    pub fn verify_transaction(&self, tx_hash: &str) -> bool {
        let leaves = self.merkle_leaves();
        let Some(index) = leaves.iter().position(|h| h == tx_hash) else {
            return false;
        };
        match merkle::get_proof(&leaves, index) {
            Some(proof) => {
                merkle::verify_proof(tx_hash, &proof, self.merkle_root.as_deref().unwrap_or(""))
            }
            None => false,
        }
    }

    /// Size của block (JSON bytes)
    pub fn get_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }
}

/// Port Block.toString() — hộp hiển thị đẹp trong CLI.
impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::util::{BRIGHT, CYAN, DIM, GREEN, MAGENTA, RESET, YELLOW};

        let miner_addr = self
            .miner_address
            .as_deref()
            .map(util::shorten_address)
            .unwrap_or_else(|| "None".to_string());
        let reward = self.coinbase_tx.as_ref().map(|c| c.amount).unwrap_or(0);

        let tx_display = if !self.transactions.is_empty() {
            self.transactions
                .iter()
                .enumerate()
                .map(|(i, tx)| {
                    format!(
                        "  {}. {} → {} ({})",
                        i + 1,
                        util::shorten_address(&tx.from),
                        util::shorten_address(&tx.to),
                        util::fmt_micro(tx.amount)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else if let Some(data) = &self.data {
            format!("  {DIM}{data}{RESET}")
        } else {
            format!("  {DIM}No transactions{RESET}")
        };

        let hash_short = if self.hash.len() > 28 {
            format!("{}...{}", &self.hash[..20], &self.hash[self.hash.len() - 8..])
        } else {
            self.hash.clone()
        };

        write!(
            f,
            "\n{CYAN}╔══════════════════════════════════════════════════════╗{RESET}\n\
             {CYAN}║{RESET}  {BRIGHT}{YELLOW}⬛ Block #{index}{RESET}                                         {CYAN}║{RESET}\n\
             {CYAN}╠══════════════════════════════════════════════════════╣{RESET}\n\
             {CYAN}║{RESET}  {DIM}Hash{RESET}          : {hash_short}\n\
             {CYAN}║{RESET}  {DIM}Previous{RESET}      : {}...\n\
             {CYAN}║{RESET}  {DIM}MerkleRoot{RESET}    : {}...\n\
             {CYAN}║{RESET}  {DIM}Timestamp{RESET}     : {ts}\n\
             {CYAN}║{RESET}  {DIM}Nonce{RESET}         : {MAGENTA}{nonce}{RESET}\n\
             {CYAN}╠──────────────────────────────────────────────────────╣{RESET}\n\
             {CYAN}║{RESET}  {DIM}Miner{RESET}         : {CYAN}{miner_addr}{RESET}\n\
             {CYAN}║{RESET}  {DIM}Reward{RESET}        : {GREEN}+{reward}{RESET} coins\n\
             {CYAN}║{RESET}  {DIM}Transactions{RESET}  : {YELLOW}{tx_count}{RESET}\n\
             {CYAN}║{RESET}  {DIM}Total Fees{RESET}    : {GREEN}+{fees}{RESET} coins\n\
             {CYAN}╠──────────────────────────────────────────────────────╣{RESET}\n\
             {CYAN}║{RESET}  {BRIGHT}Transactions:{RESET}\n\
             {tx_display}\n\
             {CYAN}╚══════════════════════════════════════════════════════╝{RESET}",
            &self.previous_hash[..28.min(self.previous_hash.len())],
            self.merkle_root.as_deref().map(|m| &m[..28.min(m.len())]).unwrap_or(""),
            index = self.index,
            ts = util::fmt_timestamp(self.timestamp),
            nonce = self.nonce,
            reward = util::fmt_micro(reward),
            tx_count = self.transactions.len(),
            fees = util::fmt_micro(self.total_fees),
        )
    }
}
