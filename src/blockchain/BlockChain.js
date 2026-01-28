const { Block } = require("./Block");
const { BalanceTracker } = require("../wallet/BalanceTracker.js");
const { shortenAddress } = require("../util/AddressHelper.js");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

class BlockChain {
  constructor(difficulty = BLOCKCHAIN_CONSTANTS.DEFAULT_DIFFICULTY) {
    this.chain = [Block.genesis()];
    this.difficulty = difficulty;
    this.mempool = [];
    this.balanceTracker = new BalanceTracker();
    this.spentTxids = new Set(); // Track spent transactions (double spend protection)
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
      console.log(`Difficulty increased to ${this.difficulty}`);
    } else if (timeTaken > timeExpected * 2) {
      // Mining quá chậm -> giảm difficulty
      this.difficulty = Math.max(
        this.difficulty - 1,
        BLOCKCHAIN_CONSTANTS.MIN_DIFFICULTY,
      );
      console.log(`Difficulty decreased to ${this.difficulty}`);
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

    // Kiểm tra block có hợp lệ không
    if (block.index !== latestBlock.index + 1) {
      console.log(
        `Block index mismatch. Expected ${latestBlock.index + 1}, got ${
          block.index
        }`,
      );
      return false;
    }

    if (block.previousHash !== latestBlock.hash) {
      console.log("Block previousHash doesn't match latest block hash");
      return false;
    }

    // Recreate block object để có đầy đủ methods
    const receivedBlock = this._recreateBlock(block);

    // Verify hash
    if (receivedBlock.calculateHash() !== block.hash) {
      console.log("Block hash is invalid");
      return false;
    }

    // Verify difficulty
    if (!block.hash.startsWith("0".repeat(this.difficulty))) {
      console.log("Block doesn't meet difficulty requirement");
      return false;
    }

    this.chain.push(receivedBlock);

    // Update balance tracker after adding block
    this.balanceTracker.updateBalance(this.chain);

    // Xóa các transactions đã được confirm khỏi mempool
    this._removeConfirmedTransactions(receivedBlock);

    console.log(`Block #${block.index} accepted and added to chain`);
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

    this.mempool = this.mempool.filter((mempoolTx) => {
      // Kiểm tra xem tx trong mempool có match với tx trong block không
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
      console.log(`  Removed ${removedCount} confirmed tx(s) from mempool`);
    }
  }

  isChainValid(chain = null) {
    const chainToValidate = chain || this.chain;

    for (let i = 1; i < chainToValidate.length; i++) {
      const currentBlock = chainToValidate[i];
      const previousBlock = chainToValidate[i - 1];

      // Recreate block nếu cần để có method calculateHash
      const blockToCheck = this._recreateBlock(currentBlock);

      // Check xem bị sửa chưa
      if (blockToCheck.hash !== blockToCheck.calculateHash()) {
        return false;
      }
      // Check liên kết có bị phá vỡ không
      if (blockToCheck.previousHash !== previousBlock.hash) {
        return false;
      }
      // check độ khó
      if (!blockToCheck.hash.startsWith("0".repeat(this.difficulty))) {
        return false;
      }
    }
    return true;
  }

  receiveChain(newChain) {
    if (newChain.length <= this.chain.length) {
      console.log("Received chain is not longer than current chain.");
      return false;
    }

    if (!this.isChainValid(newChain)) {
      console.log("Received chain is invalid.");
      return false;
    }

    console.log(
      `Replacing current chain (${this.chain.length} blocks) with new chain (${newChain.length} blocks)`,
    );

    // Convert plain objects to Block instances
    this.chain = newChain.map((blockData) => this._recreateBlock(blockData));

    // Update balance tracker after replacing chain
    this.balanceTracker.updateBalance(this.chain);

    // Reset mempool khi nhận chain mới vì các tx cũ có thể không còn valid
    this.mempool = [];
    console.log("Mempool cleared after receiving new chain");

    return true;
  }

  addToMempool(transaction) {
    this.mempool.push(transaction);
  }

  mineMempool(minerAddress = "SYSTEM") {
    if (this.mempool.length === 0) {
      console.log("Mempool is empty, nothing to mine.");
      return;
    }
    const block = this.mineBlock(minerAddress, null);
    console.log("Block mined successfully!");
    return block;
  }

  // thêm validate và transaction vào mempool
  addTransaction(transaction) {
    // 1. Kiểm tra txid duplicate (double spend trong mempool)
    if (transaction.txid) {
      const existsInMempool = this.mempool.some(
        (tx) => tx.txid === transaction.txid,
      );
      if (existsInMempool) {
        throw new Error(
          "Transaction already exists in mempool (duplicate txid)",
        );
      }

      // Kiểm tra double spend trong blockchain
      if (this.spentTxids.has(transaction.txid)) {
        throw new Error("Transaction already spent (double spend attempt)");
      }
    }

    // 2. Kiểm tra duplicate cũ (fallback)
    const isDuplicate = this.mempool.some(
      (tx) =>
        tx.from === transaction.from &&
        tx.to === transaction.to &&
        tx.amount === transaction.amount &&
        tx.timestamp === transaction.timestamp,
    );

    if (isDuplicate) {
      throw new Error("Transaction already exists in mempool");
    }

    // 3. Validate signature
    if (transaction.type === "TRANSFER") {
      if (!transaction.isValid()) {
        throw new Error("Invalid transaction signature");
      }
    }

    // 4. Kiểm tra mempool size limit
    if (
      this.mempool.length >=
      BLOCKCHAIN_CONSTANTS.MAX_TRANSACTIONS_PER_BLOCK * 2
    ) {
      throw new Error("Mempool is full. Please wait or increase fee.");
    }

    // 5. Validate balance
    const currentBalance = this.balanceTracker.getBalance(transaction.from);
    const pendingAmount = this.mempool
      .filter((tx) => tx.from === transaction.from)
      .reduce((sum, tx) => sum + tx.getTotalCost(), 0);

    const totalCost = transaction.getTotalCost();
    const availableBalance = currentBalance - pendingAmount;

    if (availableBalance < totalCost) {
      throw new Error(
        `Insufficient balance. Current: ${currentBalance}, Pending: ${pendingAmount}, Available: ${availableBalance}, Required: ${totalCost}`,
      );
    }

    this.mempool.push(transaction);
    console.log(
      "Transaction added to mempool. mempool size:",
      this.mempool.length,
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

    // Update balances
    this.balanceTracker.updateBalance(this.chain);

    // Điều chỉnh difficulty
    this.adjustDifficulty();

    const displayAddress =
      minerAddress === "SYSTEM" ? "SYSTEM" : shortenAddress(minerAddress);
    console.log(
      `Block #${newBlock.index} mined successfully by ${displayAddress}`,
    );
    return newBlock;
  }

  getBalance(address) {
    return this.balanceTracker.getBalance(address);
  }

  getAllBalances() {
    return this.balanceTracker.getAllBalances();
  }

  getMiningReward() {
    const latestBlock = this.getLatestBlock();
    return latestBlock.coinbaseTx ? latestBlock.coinbaseTx.amount : 0;
  }

  addBlock(data) {
    return this.mineBlock("SYSTEM", data);
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
    return Math.floor(
      BLOCKCHAIN_CONSTANTS.INITIAL_MINING_REWARD / Math.pow(2, halvings),
    );
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
