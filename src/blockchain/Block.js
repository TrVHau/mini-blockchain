const crypto = require("crypto");
class Block {
  constructor(index, data, previousHash) {
    this.index = index;
    this.data = data;
    this.previousHash = previousHash;
    this.timestamp = Date.now();
    this.nonce = 0;
    this.hash = this.calculateHash();
  }
  calculateHash() {
    return crypto
      .createHash("sha256")
      .update(
        `${this.index}|${this.previousHash}|${this.timestamp}|${
          this.nonce
        }|${JSON.stringify(this.data)}`
      )
      .digest("hex");
  }

  static genesis() {
    return new Block(0, "Genesis Block", "0");
  }
  mineBlock(difficulty) {
    const target = "0".repeat(difficulty);
    let attempts = 0;
    while (!this.hash.startsWith(target)) {
      this.nonce++;
      attempts++;
      this.hash = this.calculateHash();
    }
    console.log(`Block mined: ${this.hash} in ${attempts} attempts`);
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
