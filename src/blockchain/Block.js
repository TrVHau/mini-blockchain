const { CoinbaseTransaction } = require("./CoinbaseTransaction.js");
const { shortenAddress } = require("../util/AddressHelper.js");
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
        `${this.index}|${this.previousHash}|${this.timestamp}|${this.nonce}|${this.data}|${txData}`
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

  getTotalFees() {
    return this.totalFees;
  }

  toString() {
    const minerAddr = this.minerAddress
      ? shortenAddress(this.minerAddress)
      : "None";

    const reward = this.coinbaseTx ? this.coinbaseTx.amount : 0;

    // Format transactions for display with shortened addresses
    let txDisplay = this.data;

    // Nếu có transactions trong block, hiển thị chúng
    if (this.transactions && this.transactions.length > 0) {
      txDisplay = this.transactions.map((tx) => ({
        from: tx.from ? shortenAddress(tx.from) : tx.from,
        to: tx.to ? shortenAddress(tx.to) : tx.to,
        amount: tx.amount,
        fee: tx.fee || 0,
        type: tx.type,
      }));
    } else if (Array.isArray(this.data) && this.data.length > 0) {
      // Fallback: nếu data là array (legacy)
      txDisplay = this.data.map((tx) => ({
        from: tx.from ? shortenAddress(tx.from) : tx.from,
        to: tx.to ? shortenAddress(tx.to) : tx.to,
        amount: tx.amount,
        fee: tx.fee || 0,
        type: tx.type,
      }));
    }

    return `
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Block #${this.index}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Hash          : ${this.hash}
Previous Hash : ${this.previousHash}
Timestamp     : ${new Date(this.timestamp).toLocaleString()}
Nonce         : ${this.nonce}
Miner Address : ${minerAddr}
Mining Reward : ${reward} coins
Transactions  : ${this.transactions.length}
Total Fees    : ${this.totalFees} coins
Data          : ${JSON.stringify(txDisplay)}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`;
  }
}
exports.Block = Block;
