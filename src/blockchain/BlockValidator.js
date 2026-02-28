const { Logger } = require("../util/Logger");
const crypto = require("crypto");

const logger = new Logger("BLOCK_VALIDATOR");

/**
 * BlockValidator - Comprehensive block validation
 */
class BlockValidator {
  constructor(blockchain) {
    this.blockchain = blockchain;
  }

  /**
   * Validate block hash
   */
  validateHash(block) {
    const calculatedHash = crypto
      .createHash("sha256")
      .update(
        `${block.index}|${block.previousHash}|${block.timestamp}|${block.nonce}|${block.merkleRoot || ""}`,
      )
      .digest("hex");

    if (calculatedHash !== block.hash) {
      logger.error(
        `Block hash mismatch. Expected: ${calculatedHash}, Got: ${block.hash}`,
      );
      return false;
    }

    return true;
  }

  /**
   * Validate proof of work
   */
  validateProofOfWork(block, difficulty) {
    const target = "0".repeat(difficulty);
    if (!block.hash.startsWith(target)) {
      logger.error(
        `Block #${block.index} does not meet difficulty ${difficulty}`,
      );
      return false;
    }
    return true;
  }

  /**
   * Validate block index
   */
  validateIndex(block, expectedIndex) {
    if (block.index !== expectedIndex) {
      logger.error(
        `Block index mismatch. Expected: ${expectedIndex}, Got: ${block.index}`,
      );
      return false;
    }
    return true;
  }

  /**
   * Validate previous hash linkage
   */
  validatePreviousHash(block, expectedPreviousHash) {
    if (block.previousHash !== expectedPreviousHash) {
      logger.error(
        `Block previousHash mismatch. Expected: ${expectedPreviousHash}, Got: ${block.previousHash}`,
      );
      return false;
    }
    return true;
  }

  /**
   * Validate block timestamp
   */
  validateTimestamp(block, previousBlock) {
    // Timestamp should not be in the future (with 2 hour tolerance)
    const now = Date.now();
    const maxFuture = now + 2 * 60 * 60 * 1000; // 2 hours

    if (block.timestamp > maxFuture) {
      logger.error(
        `Block timestamp too far in future: ${new Date(block.timestamp)}`,
      );
      return false;
    }

    // Timestamp should be after previous block
    if (previousBlock && block.timestamp < previousBlock.timestamp) {
      logger.error(
        `Block timestamp before previous block: ${new Date(block.timestamp)} < ${new Date(previousBlock.timestamp)}`,
      );
      return false;
    }

    return true;
  }

  /**
   * Validate merkle root
   */
  validateMerkleRoot(block) {
    // Skip if no merkle root
    if (!block.merkleRoot) {
      return true;
    }

    // Recalculate merkle root
    const MerkleTree = require("../util/MerkleTree");

    const txHashes = (block.transactions || []).map((tx) => {
      if (tx.txid) return tx.txid;
      if (tx.calculateHash) return tx.calculateHash();
      return MerkleTree.hash(JSON.stringify(tx));
    });

    if (block.coinbaseTx) {
      const coinbaseHash = MerkleTree.hash(
        `${block.coinbaseTx.to}|${block.coinbaseTx.amount}|${block.index}`,
      );
      txHashes.unshift(coinbaseHash);
    }

    const calculatedRoot = MerkleTree.calculateRoot(txHashes);

    if (calculatedRoot !== block.merkleRoot) {
      logger.error(
        `Merkle root mismatch. Expected: ${calculatedRoot}, Got: ${block.merkleRoot}`,
      );
      return false;
    }

    return true;
  }

  /**
   * Validate block size
   */
  validateSize(block, maxSize = 1000000) {
    const size = JSON.stringify(block).length;
    if (size > maxSize) {
      logger.error(`Block too large: ${size} bytes > ${maxSize} bytes`);
      return false;
    }
    return true;
  }

  /**
   * Validate transaction count
   */
  validateTransactionCount(block, maxTransactions = 100) {
    const txCount = block.transactions ? block.transactions.length : 0;
    if (txCount > maxTransactions) {
      logger.error(`Too many transactions: ${txCount} > ${maxTransactions}`);
      return false;
    }
    return true;
  }

  /**
   * Validate coinbase transaction
   */
  validateCoinbase(block, expectedReward, totalFees) {
    if (!block.coinbaseTx) {
      logger.error("Block missing coinbase transaction");
      return false;
    }

    const expectedAmount = expectedReward + totalFees;
    if (block.coinbaseTx.amount !== expectedAmount) {
      logger.error(
        `Invalid coinbase amount. Expected: ${expectedAmount}, Got: ${block.coinbaseTx.amount}`,
      );
      return false;
    }

    if (!block.coinbaseTx.to || block.coinbaseTx.to.length !== 64) {
      logger.error("Invalid coinbase recipient address");
      return false;
    }

    return true;
  }

  /**
   * Validate all transactions in block
   */
  validateTransactions(block, transactionValidator, validationContext) {
    if (!block.transactions || block.transactions.length === 0) {
      return true;
    }

    for (const tx of block.transactions) {
      if (!transactionValidator.validate(tx, validationContext)) {
        logger.error("Invalid transaction in block:", tx.txid || tx);
        return false;
      }
    }

    return true;
  }

  /**
   * Full block validation
   */
  validate(block, options = {}) {
    const {
      difficulty = 4,
      expectedIndex = null,
      expectedPreviousHash = null,
      previousBlock = null,
      validateTransactions = false,
      transactionValidator = null,
      validationContext = {},
      expectedReward = 0,
      maxBlockSize = 1000000,
      maxTransactions = 100,
    } = options;

    // Basic structure validation
    if (!this.validateHash(block)) return false;
    if (!this.validateProofOfWork(block, difficulty)) return false;
    if (!this.validateSize(block, maxBlockSize)) return false;
    if (!this.validateTransactionCount(block, maxTransactions)) return false;

    // Merkle root validation
    if (!this.validateMerkleRoot(block)) return false;

    // Index and linkage validation
    if (expectedIndex !== null) {
      if (!this.validateIndex(block, expectedIndex)) return false;
    }

    if (expectedPreviousHash !== null) {
      if (!this.validatePreviousHash(block, expectedPreviousHash)) return false;
    }

    // Timestamp validation
    if (!this.validateTimestamp(block, previousBlock)) return false;

    // Coinbase validation
    if (block.coinbaseTx) {
      const totalFees =
        block.transactions?.reduce((sum, tx) => sum + (tx.fee || 0), 0) || 0;
      if (!this.validateCoinbase(block, expectedReward, totalFees))
        return false;
    }

    // Transaction validation (optional, expensive)
    if (validateTransactions && transactionValidator) {
      if (
        !this.validateTransactions(
          block,
          transactionValidator,
          validationContext,
        )
      ) {
        return false;
      }
    }

    logger.debug(`Block #${block.index} validation passed`);
    return true;
  }

  /**
   * Validate entire chain
   */
  validateChain(chain, difficulty = 4) {
    const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

    // Genesis block
    if (chain.length === 0) {
      logger.error("Empty chain");
      return false;
    }

    // Validate each block
    for (let i = 1; i < chain.length; i++) {
      const currentBlock = chain[i];
      const previousBlock = chain[i - 1];

      // Tính expected reward theo block height (halving)
      const halvings = Math.floor(i / BLOCKCHAIN_CONSTANTS.HALVING_INTERVAL);
      const reward =
        BLOCKCHAIN_CONSTANTS.INITIAL_MINING_REWARD / Math.pow(2, halvings);

      if (
        !this.validate(currentBlock, {
          difficulty,
          expectedIndex: i,
          expectedPreviousHash: previousBlock.hash,
          previousBlock,
          expectedReward: reward > 0 ? reward : 0,
        })
      ) {
        logger.error(`Chain validation failed at block #${i}`);
        return false;
      }
    }

    logger.info(`Chain validation passed (${chain.length} blocks)`);
    return true;
  }
}

module.exports = BlockValidator;
