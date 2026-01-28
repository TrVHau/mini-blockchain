/**
 * @fileoverview Type definitions for Mini Blockchain
 */

/**
 * @typedef {Object} TransactionData
 * @property {string} from - Sender address (64 hex chars)
 * @property {string} to - Recipient address (64 hex chars)
 * @property {number} amount - Amount to transfer
 * @property {number} fee - Transaction fee
 * @property {number} timestamp - Unix timestamp
 * @property {string} type - Transaction type ('TRANSFER' | 'COINBASE')
 * @property {string} [senderPublicKey] - PEM public key for verification
 * @property {string} [signature] - ECDSA signature
 * @property {string} [txid] - Transaction ID (hash)
 */

/**
 * @typedef {Object} BlockData
 * @property {number} index - Block index
 * @property {string} data - Block data (for genesis)
 * @property {string} previousHash - Previous block hash
 * @property {number} timestamp - Unix timestamp
 * @property {number} nonce - PoW nonce
 * @property {string} hash - Block hash
 * @property {string} [minerAddress] - Miner's address
 * @property {Object} [coinbaseTx] - Coinbase transaction
 * @property {TransactionData[]} transactions - Block transactions
 * @property {number} totalFees - Sum of transaction fees
 * @property {string} [merkleRoot] - Merkle root of transactions
 */

/**
 * @typedef {Object} WalletData
 * @property {string} publicKey - PEM public key
 * @property {string} privateKey - PEM private key
 * @property {string} address - Hash address (64 hex chars)
 */

/**
 * @typedef {Object} PeerInfo
 * @property {string} address - WebSocket address
 * @property {string} state - Connection state
 */

/**
 * @typedef {Object} BlockchainStats
 * @property {number} totalBlocks - Number of blocks
 * @property {number} totalTransactions - Total transactions
 * @property {number} totalCoins - Total coins in circulation
 * @property {number} difficulty - Current difficulty
 * @property {number} mempoolSize - Pending transactions
 * @property {number} avgBlockTime - Average block time (seconds)
 * @property {string} latestBlockHash - Latest block hash
 */

/**
 * @typedef {Object} TransactionResult
 * @property {TransactionData} transaction - The transaction
 * @property {number} blockIndex - Block containing the transaction
 * @property {number} confirmations - Number of confirmations
 */

module.exports = {};
