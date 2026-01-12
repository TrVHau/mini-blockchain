const { Block } = require("./Block");

class Blockchain {
  constructor(difficulty = 2) {
    this.chain = [Block.genesis()];
    this.difficulty = difficulty;
  }

  get() {
    return this.chain;
  }

  getLatesBlock() {
    return this.chain[this.chain.length - 1];
  }

  addBlock(data) {
    const preBlock = this.getLatesBlock();
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

  replaceChain(newChain) {
    if (newChain.length <= this.chain.length) {
      console.log("Received chain is not longer than current chain.");
      return false;
    }
    if (!this.isChainValid(newChain)) {
      console.log("Received chain is invalid.");
      return false;
    }
    console.log("Replacing current chain with new chain.");
    this.chain = newChain;
    return true;
  }
}
exports.Blockchain = Blockchain;
