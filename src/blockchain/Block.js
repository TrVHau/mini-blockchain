const { CoinbaseTransaction } = require("./CoinbaseTransaction.js");
const crypto = require("crypto");
class Block {
  constructor(index, data, previousHash, minerAddress = null) {
    this.index = index;
    this.data = data;
    this.previousHash = previousHash;
    this.timestamp = Date.now();
    this.nonce = 0;
    this.hash = this.calculateHash();

    this.minerAddress = minerAddress;
    this.coinbaseTx = null; //set sau
    this.transactions = [];
    this.totalFees = 0;
  }

  calculateHash() {
    const txData = JSON.stringify(this.transactions);
    return crypto
      .createHash("sha256")
      .update(
        `${this.index}|${this.previousHash}|${this.timestamp}
        |${this.nonce}|${txData}|${txData}`
      )
      .digest("hex");
  }

  static genesis() {
    return new Block(0, "Genesis Block", "0");
  }

  mineBlock(difficulty, minerAddress) {
    this.totalFees = this.transactions.reduce(
      (sum, tx) => sum + (tx.fee || 0),
      0
    );

    this.coinbaseTx = new CoinbaseTransaction(minerAddress, this.index);
    const target = "0".repeat(difficulty);

    // mining loop
    while (!this.hash.startsWith(target)) {
      this.nonce++;
      this.hash = this.calculateHash();
    }
  }

  toString() {
    return `Block -
      Index       : ${this.index}
      Data        : ${JSON.stringify(this.data)}
      PreviousHash: ${this.previousHash}
      Hash        : ${this.hash}
      Timestamp   : ${this.timestamp}
      Nonce       : ${this.nonce}`;
  }
}
exports.Block = Block;
