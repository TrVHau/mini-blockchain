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
  }

  get() {
    return this.chain;
  }

  getLatestBlock() {
    return this.chain[this.chain.length - 1];
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
      blockData.minerAddress
    );
    block.timestamp = blockData.timestamp;
    block.nonce = blockData.nonce;
    block.hash = blockData.hash;
    block.coinbaseTx = blockData.coinbaseTx;
    block.transactions = blockData.transactions || [];
    block.totalFees = blockData.totalFees || 0;
    return block;
  }

  receiveBlock(block) {
    const latestBlock = this.getLatestBlock();

    // Kiểm tra block có hợp lệ không
    if (block.index !== latestBlock.index + 1) {
      console.log(
        `Block index mismatch. Expected ${latestBlock.index + 1}, got ${
          block.index
        }`
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

    console.log(`Block #${block.index} accepted and added to chain`);
    return true;
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
      `Replacing current chain (${this.chain.length} blocks) with new chain (${newChain.length} blocks)`
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
  addTransaction(transaction, sendersPublicKey) {
    // Kiểm tra xem transaction đã có trong mempool chưa
    const isDuplicate = this.mempool.some(
      (tx) =>
        tx.from === transaction.from &&
        tx.to === transaction.to &&
        tx.amount === transaction.amount &&
        tx.timestamp === transaction.timestamp
    );

    if (isDuplicate) {
      throw new Error("Transaction already exists in mempool");
    }

    // validate signature
    if (transaction.type === "TRANSFER") {
      if (!transaction.isValid(sendersPublicKey)) {
        throw new Error("Invalid transaction signature");
      }
    }

    // validate balance - phải tính cả pending transactions trong mempool
    const currentBalance = this.balanceTracker.getBalance(transaction.from);

    // Tính tổng số tiền đang pending trong mempool từ cùng address
    const pendingAmount = this.mempool
      .filter((tx) => tx.from === transaction.from)
      .reduce((sum, tx) => sum + tx.getTotalCost(), 0);

    const totalCost = transaction.getTotalCost();
    const availableBalance = currentBalance - pendingAmount;

    if (availableBalance < totalCost) {
      throw new Error(
        `Insufficient balance. Current: ${currentBalance}, Pending: ${pendingAmount}, Available: ${availableBalance}, Required: ${totalCost}`
      );
    }

    this.mempool.push(transaction);
    console.log(
      "Transaction added to mempool. mempool size:",
      this.mempool.length
    );
    return true;
  }

  mineBlock(minerAddress, data = null) {
    const preBlock = this.getLatestBlock();
    const newBlock = new Block(
      preBlock.index + 1,
      data,
      preBlock.hash,
      minerAddress
    );

    // add transactions from mempool
    newBlock.transactions = this.mempool;
    this.mempool = [];

    //mine block - tạo coinbase transaction bên trong
    newBlock.mineBlock(this.difficulty, minerAddress);

    this.chain.push(newBlock);

    // update balances
    this.balanceTracker.updateBalance(this.chain);

    const displayAddress =
      minerAddress === "SYSTEM" ? "SYSTEM" : shortenAddress(minerAddress);
    console.log(
      `Block #${newBlock.index} mined successfully by ${displayAddress}`
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
}
exports.BlockChain = BlockChain;
