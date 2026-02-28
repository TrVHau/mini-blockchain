const { Block } = require("./Block");
const { BalanceTracker } = require("../wallet/BalanceTracker.js");
const { shortenAddress } = require("../util/AddressHelper.js");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");
const TransactionValidator = require("./TransactionValidator.js");
const BlockValidator = require("./BlockValidator.js");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("BLOCKCHAIN");

class BlockChain {
  constructor(difficulty = BLOCKCHAIN_CONSTANTS.DEFAULT_DIFFICULTY) {
    this.chain = [Block.genesis()];
    this.difficulty = difficulty;
    this.mempool = [];
    this.balanceTracker = new BalanceTracker();
    this.spentTxids = new Set(); // Track spent transactions (double spend protection)

    // New components
    this.transactionValidator = new TransactionValidator(this);
    this.blockValidator = new BlockValidator(this);
  }

  get() {
    return this.chain;
  }

  getLatestBlock() {
    return this.chain[this.chain.length - 1];
  }

  /**
   * Lấy block theo index
   */
  getBlock(index) {
    if (index < 0 || index >= this.chain.length) return null;
    return this.chain[index];
  }

  /**
   * Tìm transaction theo txid
   */
  getTransaction(txid) {
    for (const block of this.chain) {
      if (block.transactions) {
        const tx = block.transactions.find((t) => t.txid === txid);
        if (tx) {
          return {
            transaction: tx,
            blockIndex: block.index,
            confirmations: this.getConfirmations(block.index),
          };
        }
      }
    }
    return null;
  }

  /**
   * Tính số confirmations của một block
   */
  getConfirmations(blockIndex) {
    return this.chain.length - 1 - blockIndex;
  }

  /**
   * Kiểm tra transaction đã được confirmed đủ chưa
   */
  isConfirmed(
    txid,
    requiredConfirmations = BLOCKCHAIN_CONSTANTS.CONFIRMATIONS_REQUIRED,
  ) {
    const txInfo = this.getTransaction(txid);
    if (!txInfo) return false;
    return txInfo.confirmations >= requiredConfirmations;
  }

  /**
   * Điều chỉnh difficulty dựa trên thời gian mining
   */
  adjustDifficulty() {
    const interval = BLOCKCHAIN_CONSTANTS.DIFFICULTY_ADJUSTMENT_INTERVAL;
    const latestBlock = this.getLatestBlock();

    // Chỉ điều chỉnh mỗi N blocks
    if (latestBlock.index % interval !== 0 || latestBlock.index === 0) {
      return this.difficulty;
    }

    const prevAdjustmentBlock = this.chain[this.chain.length - interval];
    const timeExpected = interval * BLOCKCHAIN_CONSTANTS.TARGET_BLOCK_TIME;
    const timeTaken = latestBlock.timestamp - prevAdjustmentBlock.timestamp;

    // Điều chỉnh difficulty
    if (timeTaken < timeExpected / 2) {
      // Mining quá nhanh -> tăng difficulty
      this.difficulty = Math.min(
        this.difficulty + 1,
        BLOCKCHAIN_CONSTANTS.MAX_DIFFICULTY,
      );
      logger.info(`Difficulty increased to ${this.difficulty}`);
    } else if (timeTaken > timeExpected * 2) {
      // Mining quá chậm -> giảm difficulty
      this.difficulty = Math.max(
        this.difficulty - 1,
        BLOCKCHAIN_CONSTANTS.MIN_DIFFICULTY,
      );
      logger.info(`Difficulty decreased to ${this.difficulty}`);
    }

    return this.difficulty;
  }

  // Helper method to recreate Block instance from plain object
  _recreateBlock(blockData) {
    if (typeof blockData.calculateHash === "function") {
      return blockData;
    }

    const block = new Block(
      blockData.index,
      blockData.data,
      blockData.previousHash,
      blockData.minerAddress,
    );
    block.timestamp = blockData.timestamp;
    block.nonce = blockData.nonce;
    block.hash = blockData.hash;
    block.coinbaseTx = blockData.coinbaseTx;
    block.transactions = blockData.transactions || [];
    block.totalFees = blockData.totalFees || 0;
    block.merkleRoot = blockData.merkleRoot || null;
    return block;
  }

  receiveBlock(block) {
    const latestBlock = this.getLatestBlock();

    // Recreate block object để có đầy đủ methods
    const receivedBlock = this._recreateBlock(block);

    // Use BlockValidator for comprehensive validation
    const isValid = this.blockValidator.validate(receivedBlock, {
      difficulty: this.difficulty,
      expectedIndex: latestBlock.index + 1,
      expectedPreviousHash: latestBlock.hash,
      previousBlock: latestBlock,
      expectedReward: this.getBlockReward(),
    });

    if (!isValid) {
      logger.error(`Block #${block.index} validation failed`);
      return false;
    }

    this.chain.push(receivedBlock);

    // Incremental balance update (chỉ block mới)
    this.balanceTracker.processBlock(receivedBlock);

    // Xóa các transactions đã được confirm khỏi mempool
    this._removeConfirmedTransactions(receivedBlock);

    logger.success(`Block #${block.index} accepted and added to chain`);
    return true;
  }

  /**
   * Xóa các transactions đã được confirm trong block khỏi mempool
   */
  _removeConfirmedTransactions(block) {
    if (!block.transactions || block.transactions.length === 0) {
      return;
    }

    const confirmedTxs = block.transactions;
    const beforeCount = this.mempool.length;

    const confirmedTxids = new Set(
      confirmedTxs.filter((tx) => tx.txid).map((tx) => tx.txid),
    );

    this.mempool = this.mempool.filter((mempoolTx) => {
      // So sánh bằng txid trước (nhanh), fallback content matching
      if (mempoolTx.txid && confirmedTxids.has(mempoolTx.txid)) return false;
      return !confirmedTxs.some(
        (confirmedTx) =>
          confirmedTx.from === mempoolTx.from &&
          confirmedTx.to === mempoolTx.to &&
          confirmedTx.amount === mempoolTx.amount &&
          confirmedTx.timestamp === mempoolTx.timestamp,
      );
    });

    const removedCount = beforeCount - this.mempool.length;
    if (removedCount > 0) {
      logger.debug(`Removed ${removedCount} confirmed tx(s) from mempool`);
    }
  }

  isChainValid(chain = null) {
    const chainToValidate = chain || this.chain;
    return this.blockValidator.validateChain(chainToValidate, this.difficulty);
  }

  receiveChain(newChain) {
    // Validate new chain
    if (!this.blockValidator.validateChain(newChain, this.difficulty)) {
      logger.error("Received chain is invalid");
      return false;
    }

    // Check if new chain is longer
    if (newChain.length <= this.chain.length) {
      logger.info(
        `Received chain is not longer than current chain (${newChain.length} <= ${this.chain.length})`,
      );
      return false;
    }

    logger.info(
      `Replacing current chain (${this.chain.length} blocks) with new chain (${newChain.length} blocks)`,
    );

    // Convert plain objects to Block instances
    this.chain = newChain.map((blockData) => this._recreateBlock(blockData));

    // Update balance tracker after replacing chain
    this.balanceTracker.updateBalance(this.chain);

    // Reset mempool khi nhận chain mới vì các tx cũ có thể không còn valid
    this.mempool = [];
    logger.info("Mempool cleared after receiving new chain");

    return true;
  }

  addToMempool(transaction) {
    this.mempool.push(transaction);
  }

  // thêm validate và transaction vào mempool
  addTransaction(transaction) {
    // Use TransactionValidator for comprehensive validation
    const isValid = this.transactionValidator.validate(transaction, {
      balanceTracker: this.balanceTracker,
      mempool: this.mempool,
      spentTxids: this.spentTxids,
    });

    if (!isValid) {
      throw new Error("Transaction validation failed");
    }

    // Kiểm tra mempool size limit
    if (
      this.mempool.length >=
      BLOCKCHAIN_CONSTANTS.MAX_TRANSACTIONS_PER_BLOCK * 2
    ) {
      throw new Error("Mempool is full. Please wait or increase fee.");
    }

    this.mempool.push(transaction);
    logger.info(
      `Transaction added to mempool. Mempool size: ${this.mempool.length}`,
    );
    return true;
  }

  mineBlock(minerAddress, data = null) {
    const preBlock = this.getLatestBlock();
    const newBlock = new Block(
      preBlock.index + 1,
      data,
      preBlock.hash,
      minerAddress,
    );

    // Chọn transactions từ mempool (ưu tiên fee cao, giới hạn số lượng)
    const sortedMempool = [...this.mempool].sort(
      (a, b) => (b.fee || 0) - (a.fee || 0),
    );
    const maxTx = BLOCKCHAIN_CONSTANTS.MAX_TRANSACTIONS_PER_BLOCK;
    const maxSize = BLOCKCHAIN_CONSTANTS.MAX_BLOCK_SIZE;

    let selectedTxs = [];
    let currentSize = 0;

    for (const tx of sortedMempool) {
      if (selectedTxs.length >= maxTx) break;

      const txSize = tx.getSize ? tx.getSize() : JSON.stringify(tx).length;
      if (currentSize + txSize > maxSize) continue;

      selectedTxs.push(tx);
      currentSize += txSize;
    }

    newBlock.transactions = selectedTxs;

    // Xóa các tx đã chọn khỏi mempool
    const selectedSet = new Set(
      selectedTxs.map((tx) => tx.txid || tx.timestamp),
    );
    this.mempool = this.mempool.filter(
      (tx) => !selectedSet.has(tx.txid || tx.timestamp),
    );

    // Mine block
    newBlock.mineBlock(this.difficulty, minerAddress);

    this.chain.push(newBlock);

    // Track spent transactions
    for (const tx of newBlock.transactions) {
      if (tx.txid) {
        this.spentTxids.add(tx.txid);
      }
    }

    // Incremental balance update
    this.balanceTracker.processBlock(newBlock);

    // Điều chỉnh difficulty
    this.adjustDifficulty();

    const displayAddress =
      minerAddress === "SYSTEM" ? "SYSTEM" : shortenAddress(minerAddress);
    logger.success(`Block #${newBlock.index} mined by ${displayAddress}`);
    return newBlock;
  }

  getBalance(address) {
    return this.balanceTracker.getBalance(address);
  }

  getAllBalances() {
    return this.balanceTracker.getAllBalances();
  }

  getTransactionsHistory(address) {
    const history = [];

    this.chain.forEach((block) => {
      // kiểm tra coinbase transaction
      if (block.coinbaseTx && block.coinbaseTx.to === address) {
        history.push({
          type: "MINING_REWARD",
          from: "SYSTEM",
          to: block.coinbaseTx.to,
          amount: block.coinbaseTx.amount,
          fee: 0,
          timestamp: block.timestamp,
          blockIndex: block.index,
          hash: block.hash.substring(0, 16) + "...",
        });
      }
      // kiểm tra các transaction trong block
      if (block.transactions && block.transactions.length > 0) {
        block.transactions.forEach((tx) => {
          if (tx.from === address || tx.to === address) {
            history.push({
              type: tx.from === address ? "SENT" : "RECEIVED",
              from: tx.from,
              to: tx.to,
              amount: tx.amount,
              fee: tx.fee || 0,
              timestamp: block.timestamp,
              blockIndex: block.index,
              hash: block.hash.substring(0, 16) + "...",
            });
          }
        });
      }
    });
    return history;
  }

  /**
   * Tính block reward hiện tại (có halving)
   */
  getBlockReward() {
    const halvings = Math.floor(
      this.chain.length / BLOCKCHAIN_CONSTANTS.HALVING_INTERVAL,
    );
    const reward =
      BLOCKCHAIN_CONSTANTS.INITIAL_MINING_REWARD / Math.pow(2, halvings);
    return reward > 0 ? reward : 0;
  }

  /**
   * Lấy thống kê blockchain
   */
  getStats() {
    const chain = this.chain;
    const totalBlocks = chain.length;
    const totalTransactions = chain.reduce(
      (sum, block) => sum + (block.transactions?.length || 0),
      0,
    );
    const totalCoins = Object.values(
      this.balanceTracker.getAllBalances(),
    ).reduce((sum, bal) => sum + bal, 0);

    // Tính average block time (last 10 blocks)
    let avgBlockTime = 0;
    if (chain.length > 1) {
      const recentBlocks = chain.slice(-Math.min(10, chain.length));
      const timeDiffs = [];
      for (let i = 1; i < recentBlocks.length; i++) {
        timeDiffs.push(
          recentBlocks[i].timestamp - recentBlocks[i - 1].timestamp,
        );
      }
      if (timeDiffs.length > 0) {
        avgBlockTime = timeDiffs.reduce((a, b) => a + b, 0) / timeDiffs.length;
      }
    }

    return {
      totalBlocks,
      totalTransactions,
      totalCoins,
      difficulty: this.difficulty,
      mempoolSize: this.mempool.length,
      avgBlockTime: Math.round(avgBlockTime / 1000), // seconds
      latestBlockHash: this.getLatestBlock().hash,
      spentTxCount: this.spentTxids.size,
    };
  }

  /**
   * Tìm kiếm block theo hash
   */
  getBlockByHash(hash) {
    return this.chain.find((block) => block.hash === hash);
  }

  /**
   * Lấy danh sách transactions trong mempool, sắp xếp theo fee
   */
  getPendingTransactions() {
    return [...this.mempool].sort((a, b) => (b.fee || 0) - (a.fee || 0));
  }

  /**
   * Ước tính fee để transaction được xử lý nhanh
   */
  estimateFee() {
    if (this.mempool.length === 0) return 0;

    // Lấy fee trung bình của mempool
    const fees = this.mempool.map((tx) => tx.fee || 0);
    const avgFee = fees.reduce((a, b) => a + b, 0) / fees.length;

    // Đề xuất fee cao hơn 10%
    return Math.ceil(avgFee * 1.1);
  }
}
exports.BlockChain = BlockChain;
