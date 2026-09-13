//! Storage — port từ src/storage/Storage.js. Lưu blockchain + mempool JSON
//! cho mỗi node.

use std::path::PathBuf;

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;

pub struct Storage {
    #[allow(dead_code)]
    pub node_id: String,
    data_dir: PathBuf,
}

impl Storage {
    pub fn with_base_dir(base: &std::path::Path, node_id: &str) -> Self {
        let data_dir = base.join("nodes").join(node_id);
        let _ = std::fs::create_dir_all(&data_dir);
        Self {
            node_id: node_id.to_string(),
            data_dir,
        }
    }

    fn blockchain_path(&self) -> PathBuf {
        self.data_dir.join("blockchain.json")
    }

    fn mempool_path(&self) -> PathBuf {
        self.data_dir.join("mempool.json")
    }

    pub fn save_blockchain(&self, chain: &[Block]) -> bool {
        self.save_json(self.blockchain_path(), chain, "blockchain")
    }

    pub fn load_blockchain(&self) -> Option<Vec<Block>> {
        self.load_json(self.blockchain_path(), "blockchain")
    }

    /// Lưu mempool — tx pending sống sót qua restart
    pub fn save_mempool(&self, mempool: &[Transaction]) -> bool {
        self.save_json(self.mempool_path(), mempool, "mempool")
    }

    pub fn load_mempool(&self) -> Option<Vec<Transaction>> {
        self.load_json(self.mempool_path(), "mempool")
    }

    /// Ghi atomic: write file tạm + rename — crash giữa chừng không làm hỏng file cũ
    fn save_json<T: serde::Serialize + ?Sized>(&self, path: PathBuf, data: &T, what: &str) -> bool {
        let json = match serde_json::to_string(data) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("[STORAGE] ✗ Error serializing {what}: {e}");
                return false;
            }
        };
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)) {
            eprintln!("[STORAGE] ✗ Error saving {what}: {e}");
            return false;
        }
        true
    }

    fn load_json<T: serde::de::DeserializeOwned>(&self, path: PathBuf, what: &str) -> Option<T> {
        let content = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str(&content) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("[STORAGE] ✗ Error loading {what}: {e}");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::chain::BlockChain;
    use crate::crypto;

    /// Temp dir riêng cho từng test (test chạy song song trong cùng process)
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::env::temp_dir().join(format!("mbc-{tag}-{}-{n}", std::process::id()))
    }

    #[test]
    fn save_load_roundtrip_and_corrupted_file() {
        let dir = temp_dir("storage");
        let _ = std::fs::remove_dir_all(&dir);
        let storage = Storage::with_base_dir(&dir, "n1");

        // Chưa có file -> None
        assert!(storage.load_blockchain().is_none());

        let (_, pk) = crypto::generate_keypair();
        let miner = crypto::address_from_public_hex(&pk).unwrap();
        let mut bc = BlockChain::with_difficulty(1);
        bc.mine_block(&miner);
        assert!(storage.save_blockchain(&bc.chain));

        let loaded = storage.load_blockchain().expect("load sau khi save");
        assert_eq!(loaded.len(), bc.chain.len());
        assert_eq!(loaded[1].hash, bc.chain[1].hash);

        // File hỏng -> None (không panic)
        std::fs::write(
            dir.join("nodes").join("n1").join("blockchain.json"),
            "not json",
        )
        .unwrap();
        assert!(storage.load_blockchain().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mempool_roundtrip() {
        use crate::blockchain::transaction::Transaction;

        let dir = temp_dir("mempool");
        let _ = std::fs::remove_dir_all(&dir);
        let storage = Storage::with_base_dir(&dir, "n1");

        assert!(storage.load_mempool().is_none());

        // Tx thô (không ký) — storage là dumb serde, vẫn save/load được
        let raw = Transaction::new(&"a".repeat(64), &"b".repeat(64), 1_000_000, 0);
        assert!(storage.save_mempool(std::slice::from_ref(&raw)));
        let loaded = storage.load_mempool().expect("load sau khi save");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].amount, 1_000_000);

        // Node restart: tx không hợp lệ bị re-validate loại bỏ
        let node = crate::node::Node::with_base_dir(&dir, "n1");
        assert_eq!(node.blockchain.mempool.len(), 0);

        // Tx đã ký + đủ số dư: sống sót qua restart
        let (sk, pk) = crypto::generate_keypair();
        let from = crypto::address_from_public_hex(&pk).unwrap();
        {
            let mut n1 = crate::node::Node::with_base_dir(&dir, "n1");
            n1.blockchain.mine_block(&from);
            n1.save_state();
            let mut tx = Transaction::new(&from, &"b".repeat(64), 1_000_000, 0);
            tx.sign(&sk, &pk).unwrap();
            n1.add_transaction(&tx).unwrap();
        }
        let node = crate::node::Node::with_base_dir(&dir, "n1");
        assert_eq!(node.blockchain.mempool.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
