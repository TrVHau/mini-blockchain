// Blockchain Configuration Constants

const BLOCKCHAIN_CONSTANTS = {
  // Mining Configuration
  DEFAULT_DIFFICULTY: 4,
  MIN_DIFFICULTY: 1,
  MAX_DIFFICULTY: 6,

  // Mining Rewards
  INITIAL_MINING_REWARD: 50,
  HALVING_INTERVAL: 100, // Blocks between halving events

  // Genesis Block
  GENESIS_TIMESTAMP: 1640000000000, // Fixed: Dec 20, 2021
  GENESIS_DATA: "Genesis Block",
  GENESIS_PREVIOUS_HASH: "0",

  // Transaction
  DEFAULT_TRANSACTION_FEE: 0,

  // Network
  WEBSOCKET_HANDSHAKE_TIMEOUT: 5000,
};

module.exports = BLOCKCHAIN_CONSTANTS;
