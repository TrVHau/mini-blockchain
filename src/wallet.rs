//! Wallet + BalanceTracker — port từ src/wallet/WalletManager.js và BalanceTracker.js.
//! Keys lưu hex (thay PEM của JS).

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::blockchain::block::Block;
use crate::crypto;
use crate::util;

// ---- BalanceTracker ----

/// Số dư theo địa chỉ. i128 vì debit có thể vượt credit (giống JS cho số âm).
#[derive(Debug, Default)]
pub struct BalanceTracker {
    balances: HashMap<String, i128>,
}

impl BalanceTracker {
    /// Rebuild toàn bộ balance từ chain (dùng khi receive_chain / reset)
    pub fn update_balance(&mut self, chain: &[Block]) {
        self.balances.clear();
        for block in chain {
            self.process_block(block);
        }
    }

    /// Incremental update từ 1 block mới
    pub fn process_block(&mut self, block: &Block) {
        if let Some(coinbase) = &block.coinbase_tx {
            self.credit(&coinbase.to, coinbase.amount);
        }
        for tx in &block.transactions {
            self.debit(&tx.from, tx.total_cost());
            self.credit(&tx.to, tx.amount);
        }
    }

    pub fn credit(&mut self, address: &str, amount: u64) {
        *self.balances.entry(address.to_string()).or_insert(0) += amount as i128;
    }

    pub fn debit(&mut self, address: &str, amount: u64) {
        *self.balances.entry(address.to_string()).or_insert(0) -= amount as i128;
    }

    pub fn get_balance(&self, address: &str) -> i128 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    pub fn get_all_balances(&self) -> &HashMap<String, i128> {
        &self.balances
    }
}

// ---- WalletManager ----

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    /// Public key hex (compressed)
    pub public_key: String,
    /// Private key hex — KEEP SECRET
    pub private_key: String,
    /// Hash address (64 hex chars)
    pub address: String,
}

pub struct WalletManager {
    wallets: HashMap<String, Wallet>,
    wallet_dir: PathBuf,
}

impl WalletManager {
    pub fn new(node_id: &str) -> Self {
        let wallet_dir = if node_id == "default" {
            PathBuf::from("data")
        } else {
            PathBuf::from("data").join("nodes").join(node_id).join("wallets")
        };
        let mut manager = Self { wallets: HashMap::new(), wallet_dir };
        let _ = std::fs::create_dir_all(&manager.wallet_dir);
        manager.load_all_wallets();
        manager
    }

    /// Tạo wallet mới, lưu file, trả về hash address
    pub fn create_wallet(&mut self, name: &str) -> Result<String, String> {
        util::validate_wallet_name(name)?;
        if self.wallets.contains_key(name) {
            return Err("Wallet with this name already exists".to_string());
        }
        let (private_key, public_key) = crypto::generate_keypair();
        let address = crypto::address_from_public_hex(&public_key)?;
        let wallet = Wallet { public_key, private_key, address: address.clone() };
        self.wallets.insert(name.to_string(), wallet.clone());
        self.save_wallet_file(name, &wallet)?;
        Ok(address)
    }

    /// Import wallet từ private key hex
    pub fn import_wallet(&mut self, name: &str, private_key_hex: &str) -> Result<String, String> {
        util::validate_wallet_name(name)?;
        if self.wallets.contains_key(name) {
            return Err("Wallet with this name already exists".to_string());
        }
        let public_key = crypto::private_to_public(private_key_hex)?;
        let address = crypto::address_from_public_hex(&public_key)?;
        let wallet = Wallet {
            public_key,
            private_key: private_key_hex.to_lowercase(),
            address: address.clone(),
        };
        self.wallets.insert(name.to_string(), wallet.clone());
        self.save_wallet_file(name, &wallet)?;
        Ok(address)
    }

    pub fn delete_wallet(&mut self, name: &str) -> Result<(), String> {
        self.get(name)?; // lỗi nếu không tồn tại
        self.wallets.remove(name);
        let path = self.wallet_dir.join(format!("{name}.json"));
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    pub fn has_wallet(&self, name: &str) -> bool {
        self.wallets.contains_key(name)
    }

    pub fn list_wallets(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Hash address (64 hex chars) — dùng cho transactions
    pub fn get_address(&self, name: &str) -> Result<String, String> {
        Ok(self.get(name)?.address.clone())
    }

    pub fn get_public_key_hex(&self, name: &str) -> Result<String, String> {
        Ok(self.get(name)?.public_key.clone())
    }

    pub fn get_private_key(&self, name: &str) -> Result<String, String> {
        Ok(self.get(name)?.private_key.clone())
    }

    fn get(&self, name: &str) -> Result<&Wallet, String> {
        self.wallets.get(name).ok_or_else(|| "Wallet not found".to_string())
    }

    fn save_wallet_file(&self, name: &str, wallet: &Wallet) -> Result<(), String> {
        let path = self.wallet_dir.join(format!("{name}.json"));
        serde_json::to_string_pretty(wallet)
            .map_err(|e| e.to_string())
            .and_then(|json| std::fs::write(path, json).map_err(|e| e.to_string()))
    }

    fn load_all_wallets(&mut self) {
        let Ok(entries) = std::fs::read_dir(&self.wallet_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                // Bỏ qua wallet file hỏng, giống JS
                if let Ok(wallet) = serde_json::from_str::<Wallet>(
                    &std::fs::read_to_string(&path).unwrap_or_default(),
                ) {
                    self.wallets.insert(name.to_string(), wallet);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::transaction::CoinbaseTransaction;
    use crate::blockchain::transaction::Transaction;

    fn addr(tag: char) -> String {
        std::iter::repeat(tag).take(64).collect()
    }

    #[test]
    fn balance_tracking() {
        let mut tracker = BalanceTracker::default();
        let mut block = Block::new(1, None, "prev", Some(addr('a')));
        block.coinbase_tx = Some(CoinbaseTransaction::new(&addr('a'), 1, 0));
        block.transactions = vec![Transaction::new(&addr('a'), &addr('b'), 5_000_000, 100)];
        tracker.process_block(&block);

        assert_eq!(tracker.get_balance(&addr('a')), 16_000_000 - 5_000_100);
        assert_eq!(tracker.get_balance(&addr('b')), 5_000_000);
    }

    #[test]
    fn wallet_manager_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mbc-wallet-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let mut wm = WalletManager::new("default");
        let address = wm.create_wallet("alice").unwrap();
        assert_eq!(address.len(), 64);
        assert!(wm.has_wallet("alice"));
        let sk = wm.get_private_key("alice").unwrap();

        // Reload từ disk
        let wm2 = WalletManager::new("default");
        assert!(wm2.has_wallet("alice"));
        assert_eq!(wm2.get_address("alice").unwrap(), address);

        // Import từ private key
        let mut wm3 = WalletManager::new("default");
        let imported = wm3.import_wallet("bob", &sk).unwrap();
        assert_eq!(imported, address);

        std::env::set_current_dir("/").ok();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
