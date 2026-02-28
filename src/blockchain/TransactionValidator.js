const { Logger } = require("../util/Logger");
const crypto = require("crypto");

const logger = new Logger("TX_VALIDATOR");

/**
 * TransactionValidator - Comprehensive transaction validation
 */
class TransactionValidator {
  constructor(blockchain) {
    this.blockchain = blockchain;
  }

  /**
   * Validate transaction signature
   */
  validateSignature(transaction) {
    if (!transaction.signature || !transaction.senderPublicKey) {
      logger.error("Transaction missing signature or public key");
      return false;
    }

    try {
      const verify = crypto.createVerify("SHA256");
      verify.update(transaction.calculateHash());
      verify.end();
      return verify.verify(
        transaction.senderPublicKey,
        transaction.signature,
        "hex",
      );
    } catch (err) {
      logger.error("Signature verification failed:", err.message);
      return false;
    }
  }

  /**
   * Validate transaction amount
   */
  validateAmount(transaction) {
    if (
      typeof transaction.amount !== "number" ||
      transaction.amount <= 0 ||
      !Number.isFinite(transaction.amount)
    ) {
      logger.error("Invalid transaction amount:", transaction.amount);
      return false;
    }
    return true;
  }

  /**
   * Validate transaction fee
   */
  validateFee(transaction) {
    const fee = transaction.fee || 0;
    if (typeof fee !== "number" || fee < 0 || !Number.isFinite(fee)) {
      logger.error("Invalid transaction fee:", fee);
      return false;
    }
    return true;
  }

  /**
   * Validate addresses
   */
  validateAddresses(transaction) {
    // Check from address
    if (!this._isValidAddress(transaction.from)) {
      logger.error("Invalid 'from' address:", transaction.from);
      return false;
    }

    // Check to address
    if (!this._isValidAddress(transaction.to)) {
      logger.error("Invalid 'to' address:", transaction.to);
      return false;
    }

    // Cannot send to yourself
    if (transaction.from === transaction.to) {
      logger.warn("Transaction sending to self");
      // This is allowed but warned
    }

    return true;
  }

  /**
   * Check if address format is valid
   */
  _isValidAddress(address) {
    // Hash address should be 64 hex characters
    return (
      typeof address === "string" &&
      address.length === 64 &&
      /^[a-f0-9]+$/i.test(address)
    );
  }

  /**
   * Validate sender has sufficient balance
   */
  validateBalance(transaction, balanceTracker, mempoolTxs = []) {
    const currentBalance = balanceTracker.getBalance(transaction.from);

    // Calculate pending amount from mempool
    const pendingAmount = mempoolTxs
      .filter((tx) => tx.from === transaction.from)
      .reduce((sum, tx) => sum + tx.getTotalCost(), 0);

    const availableBalance = currentBalance - pendingAmount;
    const required = transaction.getTotalCost();

    if (availableBalance < required) {
      logger.error(
        `Insufficient balance. Current: ${currentBalance}, Pending: ${pendingAmount}, Available: ${availableBalance}, Required: ${required}`,
      );
      return false;
    }

    return true;
  }

  /**
   * Check for duplicate transaction
   */
  validateNotDuplicate(transaction, mempool, spentTxids) {
    // Check txid duplicate
    if (transaction.txid) {
      if (spentTxids.has(transaction.txid)) {
        logger.error("Transaction already spent (double spend)");
        return false;
      }

      if (mempool.some((tx) => tx.txid === transaction.txid)) {
        logger.error("Transaction already in mempool");
        return false;
      }
    }

    // Check duplicate by content
    const isDuplicate = mempool.some(
      (tx) =>
        tx.from === transaction.from &&
        tx.to === transaction.to &&
        tx.amount === transaction.amount &&
        tx.timestamp === transaction.timestamp,
    );

    if (isDuplicate) {
      logger.error("Duplicate transaction found in mempool");
      return false;
    }

    return true;
  }

  /**
   * Validate transaction size
   */
  validateSize(transaction, maxSize = 100000) {
    const size = JSON.stringify(transaction).length;
    if (size > maxSize) {
      logger.error(`Transaction too large: ${size} bytes > ${maxSize} bytes`);
      return false;
    }
    return true;
  }

  /**
   * Full transaction validation
   */
  validate(
    transaction,
    {
      balanceTracker,
      mempool = [],
      spentTxids = new Set(),
      skipBalanceCheck = false,
    },
  ) {
    // Basic validations
    if (!this.validateAmount(transaction)) return false;
    if (!this.validateFee(transaction)) return false;
    if (!this.validateAddresses(transaction)) return false;
    if (!this.validateSize(transaction)) return false;

    // Signature validation (skip for coinbase)
    if (transaction.type !== "COINBASE") {
      if (!this.validateSignature(transaction)) return false;
    }

    // Duplicate check
    if (!this.validateNotDuplicate(transaction, mempool, spentTxids)) {
      return false;
    }

    // Balance validation
    if (!skipBalanceCheck && transaction.type !== "COINBASE") {
      if (!this.validateBalance(transaction, balanceTracker, mempool)) {
        return false;
      }
    }

    logger.debug("Transaction validation passed");
    return true;
  }
}

module.exports = TransactionValidator;
