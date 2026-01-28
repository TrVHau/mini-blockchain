// Blockchain Configuration Constants

const BLOCKCHAIN_CONSTANTS = {
  // Mining Configuration
  DEFAULT_DIFFICULTY: 4,
  MIN_DIFFICULTY: 1,
  MAX_DIFFICULTY: 6,

  // Difficulty Adjustment
  DIFFICULTY_ADJUSTMENT_INTERVAL: 10, // Điều chỉnh mỗi 10 blocks
  TARGET_BLOCK_TIME: 30000, // 30 seconds per block (ms)

  // Mining Rewards
  INITIAL_MINING_REWARD: 16, // Initial reward for mining a block
  HALVING_INTERVAL: 50, // Blocks between halving events (tăng lên cho thực tế hơn)

  // Block Limits
  MAX_TRANSACTIONS_PER_BLOCK: 100, // Giới hạn tx mỗi block
  MAX_BLOCK_SIZE: 1000000, // 1MB max block size (bytes)

  // Transaction
  DEFAULT_TRANSACTION_FEE: 0,
  MIN_TRANSACTION_FEE: 0,

  // Confirmations
  CONFIRMATIONS_REQUIRED: 6, // Số blocks để coi là confirmed

  // Genesis Block
  GENESIS_TIMESTAMP: 1640000000000, // Fixed: Dec 20, 2021
  GENESIS_DATA: "Genesis Block",
  GENESIS_PREVIOUS_HASH: "0",

  // Network
  WEBSOCKET_HANDSHAKE_TIMEOUT: 5000,
  MAX_PEERS: 50,
};

module.exports = BLOCKCHAIN_CONSTANTS;
