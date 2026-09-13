//! Storage — port từ src/storage/Storage.js. Lưu blockchain JSON cho mỗi node.

use std::path::PathBuf;

use crate::blockchain::block::Block;

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

    pub fn save_blockchain(&self, chain: &[Block]) -> bool {
        match serde_json::to_string(chain) {
            Ok(json) => match std::fs::write(self.blockchain_path(), json) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("[STORAGE] ✗ Error saving blockchain: {e}");
                    false
                }
            },
            Err(e) => {
                eprintln!("[STORAGE] ✗ Error serializing blockchain: {e}");
                false
            }
        }
    }

    pub fn load_blockchain(&self) -> Option<Vec<Block>> {
        let path = self.blockchain_path();
        let content = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str(&content) {
            Ok(chain) => Some(chain),
            Err(e) => {
                eprintln!("[STORAGE] ✗ Error loading blockchain: {e}");
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

    #[test]
    fn save_load_roundtrip_and_corrupted_file() {
        let dir = std::env::temp_dir().join(format!("mbc-storage-test-{}", std::process::id()));
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
}
