//! Storage — port từ src/storage/Storage.js. Lưu blockchain JSON cho mỗi node.

use std::path::PathBuf;

use crate::blockchain::block::Block;

pub struct Storage {
    #[allow(dead_code)]
    pub node_id: String,
    data_dir: PathBuf,
}

impl Storage {
    pub fn new(node_id: &str) -> Self {
        let data_dir = PathBuf::from("data").join("nodes").join(node_id);
        let _ = std::fs::create_dir_all(&data_dir);
        Self { node_id: node_id.to_string(), data_dir }
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
