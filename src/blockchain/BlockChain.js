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
  isChainValid() {
    for (let i = 1; i < this.chain.length; i++) {
      const currentBlock = this.chain[i];
      const previousBlock = this.chain[i - 1];
      // check xem bi sửa chưa
      if (currentBlock.hash !== currentBlock.calculateHash()) {
        return false;
      }
      // chck liên kết có bị phá vỡ không
      if (currentBlock.previousHash !== previousBlock.hash) {
        return false;
      }
      // check độ khó
      if (!currentBlock.hash.startsWith("0".repeat(this.difficulty))) {
        return false;
      }
    }
    return true;
  }

  receiveChain(newChain) {
    if (newChain.length <= this.chain.length) {
      console.log("Received chain is not longer than current chain.");
      return;
    }
    if (!this.isChainValid(newChain)) {
      console.log("Received chain is invalid.");
      return;
    }
    console.log("Replacing current chain with new chain.");
    this.chain = newChain;
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
