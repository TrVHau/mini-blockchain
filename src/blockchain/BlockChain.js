const { Block } = require("./Block");

class BlockChain {
  constructor(difficulty = 2) {
    this.chain = [Block.genesis()];
    this.difficulty = difficulty;
    this.mempool = [];
  }

  get() {
    return this.chain;
  }

  getLatestBlock() {
    return this.chain[this.chain.length - 1];
  }

  addBlock(data) {
    const preBlock = this.getLatestBlock();
    const newBlock = new Block(preBlock.index + 1, data, preBlock.hash);
    newBlock.mineBlock(this.difficulty);
    this.chain.push(newBlock);
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
    const receivedBlock = new Block(
      block.index,
      block.data,
      block.previousHash
    );
    receivedBlock.timestamp = block.timestamp;
    receivedBlock.nonce = block.nonce;
    receivedBlock.hash = block.hash;

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
    console.log(`Block #${block.index} accepted and added to chain`);
    return true;
  }
  isChainValid(chain = null) {
    const chainToValidate = chain || this.chain;

    for (let i = 1; i < chainToValidate.length; i++) {
      const currentBlock = chainToValidate[i];
      const previousBlock = chainToValidate[i - 1];

      // Recreate block nếu cần để có method calculateHash
      let blockToCheck = currentBlock;
      if (typeof currentBlock.calculateHash !== "function") {
        blockToCheck = new Block(
          currentBlock.index,
          currentBlock.data,
          currentBlock.previousHash
        );
        blockToCheck.timestamp = currentBlock.timestamp;
        blockToCheck.nonce = currentBlock.nonce;
        blockToCheck.hash = currentBlock.hash;
      }

      // check xem bi sửa chưa
      if (blockToCheck.hash !== blockToCheck.calculateHash()) {
        return false;
      }
      // chck liên kết có bị phá vỡ không
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
    this.chain = newChain.map((blockData) => {
      if (typeof blockData.calculateHash === "function") {
        return blockData;
      }
      const block = new Block(
        blockData.index,
        blockData.data,
        blockData.previousHash
      );
      block.timestamp = blockData.timestamp;
      block.nonce = blockData.nonce;
      block.hash = blockData.hash;
      return block;
    });

    return true;
  }

  addToMempool(transaction) {
    this.mempool.push(transaction);
  }

  mineMempool() {
    if (this.mempool.length === 0) {
      console.log("Mempool is empty, nothing to mine.");
      return;
    }
    this.addBlock(this.mempool);
    console.log("Block mined successfully!");
    this.mempool = [];
  }
}
exports.BlockChain = BlockChain;
