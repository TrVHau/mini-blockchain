//! Blockchain configuration constants (port từ src/config/constants.js).
//! Đơn vị tiền: micro-coin (1 coin = 1_000_000 micro).

/// 1 coin = 1_000_000 micro
pub const MICRO_PER_COIN: u64 = 1_000_000;

// Mining Configuration
pub const DEFAULT_DIFFICULTY: usize = 4;
pub const MIN_DIFFICULTY: usize = 1;
pub const MAX_DIFFICULTY: usize = 6;

// Difficulty Adjustment
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: usize = 10; // Điều chỉnh mỗi 10 blocks
pub const TARGET_BLOCK_TIME: u64 = 30_000; // 30 seconds per block (ms)

// Mining Rewards
pub const INITIAL_MINING_REWARD: u64 = 16 * MICRO_PER_COIN; // 16 coins
pub const HALVING_INTERVAL: usize = 50; // Blocks giữa các lần halving

// Block Limits
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 100;
pub const MAX_BLOCK_SIZE: usize = 1_000_000; // 1MB max block size (bytes)

// Transaction
pub const DEFAULT_TRANSACTION_FEE: u64 = 0;

// Confirmations
pub const CONFIRMATIONS_REQUIRED: usize = 6;

// Genesis Block
pub const GENESIS_TIMESTAMP: u64 = 1_640_000_000_000; // Fixed: Dec 20, 2021
pub const GENESIS_DATA: &str = "Genesis Block";
pub const GENESIS_PREVIOUS_HASH: &str = "0";

// Network
pub const WEBSOCKET_HANDSHAKE_TIMEOUT: u64 = 5_000;
#[allow(dead_code)] // port đủ constants từ constants.js, dùng dần
pub const MAX_PEERS: usize = 50;

// Sync (SyncManager.js)
pub const SYNC_MAX_RETRY: u32 = 3;
pub const SYNC_TIMEOUT_MS: u64 = 30_000;
#[allow(dead_code)] // port đủ constants từ constants.js, dùng dần
pub const SYNC_COOLDOWN_MS: u64 = 5_000;
pub const MAX_BLOCKS_PER_REQUEST: usize = 50;

/// Reward tại block height cho trước (halving mỗi HALVING_INTERVAL blocks).
pub fn reward_at_height(height: usize) -> u64 {
    let halvings = height / HALVING_INTERVAL;
    INITIAL_MINING_REWARD.checked_shr(halvings as u32).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_math() {
        assert_eq!(reward_at_height(0), 16_000_000);
        assert_eq!(reward_at_height(49), 16_000_000);
        assert_eq!(reward_at_height(50), 8_000_000);
        assert_eq!(reward_at_height(100), 4_000_000);
        assert_eq!(reward_at_height(150), 2_000_000);
        // sau nhiều lần halving -> 0
        assert_eq!(reward_at_height(50 * 30), 0);
    }
}
