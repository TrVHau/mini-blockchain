//! Transaction — port từ src/blockchain/Transaction.js.
//! Amount/fee là micro-coin (u64).

use serde::{Deserialize, Serialize};

use crate::config::DEFAULT_TRANSACTION_FEE;
use crate::crypto;

pub const TYPE_TRANSFER: &str = "TRANSFER";
pub const TYPE_COINBASE: &str = "COINBASE";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub tx_type: String,
    /// Public key hex (compressed) để verify signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Transaction ID — set sau khi ký
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
}

impl Transaction {
    pub fn new(from: &str, to: &str, amount: u64, fee: u64) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            amount,
            fee,
            timestamp: crate::util::now_ms(),
            tx_type: TYPE_TRANSFER.to_string(),
            sender_public_key: None,
            signature: None,
            txid: None,
        }
    }

    #[allow(dead_code)] // dùng khi tái tạo coinbase từ message JSON (JS path)
    pub fn from_coinbase(coinbase: &CoinbaseTransaction) -> Self {
        Self {
            from: coinbase.from.clone(),
            to: coinbase.to.clone(),
            amount: coinbase.amount,
            fee: coinbase.fee,
            timestamp: coinbase.timestamp,
            tx_type: TYPE_COINBASE.to_string(),
            sender_public_key: None,
            signature: None,
            txid: None,
        }
    }

    pub fn total_cost(&self) -> u64 {
        self.amount.saturating_add(self.fee)
    }

    /// Hash của transaction data (không bao gồm signature)
    pub fn calculate_hash(&self) -> String {
        crypto::sha256_hex(
            format!("{}|{}|{}|{}|{}", self.from, self.to, self.amount, self.fee, self.timestamp)
                .as_bytes(),
        )
    }

    /// Transaction ID (hash bao gồm cả signature)
    pub fn calculate_txid(&self) -> String {
        crypto::sha256_hex(
            format!("{}|{}", self.calculate_hash(), self.signature.as_deref().unwrap_or("null"))
                .as_bytes(),
        )
    }

    /// Ký transaction và tạo txid
    pub fn sign(&mut self, private_key_hex: &str, public_key_hex: &str) -> Result<(), String> {
        self.sender_public_key = Some(public_key_hex.to_string());
        let tx_hash = self.calculate_hash();
        self.signature = Some(crypto::sign(private_key_hex, tx_hash.as_bytes())?);
        self.txid = Some(self.calculate_txid());
        Ok(())
    }

    /// Verify signature
    pub fn is_valid(&self) -> bool {
        match (&self.signature, &self.sender_public_key) {
            (Some(sig), Some(pk)) => crypto::verify(pk, self.calculate_hash().as_bytes(), sig),
            _ => false,
        }
    }

    /// Size để tính block size (JSON bytes)
    pub fn get_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new("", "", 0, DEFAULT_TRANSACTION_FEE)
    }
}

/// CoinbaseTransaction — port từ src/blockchain/CoinbaseTransaction.js
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoinbaseTransaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub tx_type: String,
}

impl CoinbaseTransaction {
    pub fn new(miner_address: &str, block_height: usize, total_fees: u64) -> Self {
        Self {
            from: crate::util::COINBASE_SENDER.to_string(),
            to: miner_address.to_string(),
            amount: crate::config::reward_at_height(block_height).saturating_add(total_fees),
            fee: 0,
            timestamp: crate::util::now_ms(),
            tx_type: TYPE_COINBASE.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender() -> (String, String, String) {
        let (sk, pk) = crypto::generate_keypair();
        let address = crypto::address_from_public_hex(&pk).unwrap();
        (sk, pk, address)
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let (sk, pk, addr) = sender();
        let to = "b".repeat(64);
        let mut tx = Transaction::new(&addr, &to, 5_000_000, 0);
        assert!(!tx.is_valid());

        tx.sign(&sk, &pk).unwrap();
        assert!(tx.is_valid());
        assert!(tx.txid.is_some());

        // Sửa amount -> hash đổi -> chữ ký không còn khớp
        let mut tampered = tx.clone();
        tampered.amount = 999;
        assert!(!tampered.is_valid());
    }

    #[test]
    fn txid_depends_on_signature() {
        let (sk, pk, addr) = sender();
        let to = "b".repeat(64);
        let mut tx1 = Transaction::new(&addr, &to, 1_000_000, 0);
        tx1.timestamp = 123;
        tx1.sign(&sk, &pk).unwrap();

        let mut tx2 = tx1.clone();
        tx2.signature = Some("00".to_string());
        tx2.txid = Some(tx2.calculate_txid());

        assert_ne!(tx1.txid, tx2.txid);
        // Cùng timestamp + signature -> cùng hash data
        assert_eq!(tx1.calculate_hash(), tx2.calculate_hash());
    }
}
