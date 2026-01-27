const { CoinbaseTransaction } = require("./CoinbaseTransaction.js");
const { shortenAddress } = require("../util/AddressHelper.js");
const crypto = require("crypto");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

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
        `${this.index}|${this.previousHash}|${this.timestamp}|${this.nonce}|${this.data}|${txData}`,
      )
      .digest("hex");
  }

  static genesis() {
    const genesisBlock = new Block(
      0,
      BLOCKCHAIN_CONSTANTS.GENESIS_DATA,
      BLOCKCHAIN_CONSTANTS.GENESIS_PREVIOUS_HASH,
    );
    // Set fixed timestamp for genesis block to ensure same hash across all nodes
    genesisBlock.timestamp = BLOCKCHAIN_CONSTANTS.GENESIS_TIMESTAMP;
    genesisBlock.hash = genesisBlock.calculateHash();
    return genesisBlock;
  }

  mineBlock(difficulty, minerAddress) {
    this.totalFees = this.transactions.reduce(
      (sum, tx) => sum + (tx.fee || 0),
      0,
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
    const C = {
      reset: "\x1b[0m",
      bright: "\x1b[1m",
      dim: "\x1b[2m",
      cyan: "\x1b[36m",
      yellow: "\x1b[33m",
      green: "\x1b[32m",
      magenta: "\x1b[35m",
    };

    const minerAddr = this.minerAddress
      ? shortenAddress(this.minerAddress)
      : "None";

    const reward = this.coinbaseTx ? this.coinbaseTx.amount : 0;

    // Format transactions for display with shortened addresses
    let txDisplay = "";
    if (this.transactions && this.transactions.length > 0) {
      txDisplay = this.transactions
        .map((tx, i) => {
          const from = tx.from ? shortenAddress(tx.from) : tx.from;
          const to = tx.to ? shortenAddress(tx.to) : tx.to;
          return `  ${i + 1}. ${from} → ${to} (${tx.amount} coins)`;
        })
        .join("\n");
    } else if (
      this.data &&
      typeof this.data === "string" &&
      this.data.length > 0
    ) {
      txDisplay = `  ${C.dim}${this.data}${C.reset}`;
    } else {
      txDisplay = `  ${C.dim}No transactions${C.reset}`;
    }

    const hashShort =
      this.hash.substring(0, 20) +
      "..." +
      this.hash.substring(this.hash.length - 8);

    return `
${C.cyan}╔══════════════════════════════════════════════════════╗${C.reset}
${C.cyan}║${C.reset}  ${C.bright}${C.yellow}⬛ Block #${this.index}${C.reset}                                        ${C.cyan}║${C.reset}
${C.cyan}╠══════════════════════════════════════════════════════╣${C.reset}
${C.cyan}║${C.reset}  ${C.dim}Hash${C.reset}          : ${hashShort}
${C.cyan}║${C.reset}  ${C.dim}Previous${C.reset}      : ${this.previousHash.substring(0, 28)}...
${C.cyan}║${C.reset}  ${C.dim}Timestamp${C.reset}     : ${new Date(this.timestamp).toLocaleString()}
${C.cyan}║${C.reset}  ${C.dim}Nonce${C.reset}         : ${C.magenta}${this.nonce}${C.reset}
${C.cyan}╠──────────────────────────────────────────────────────╣${C.reset}
${C.cyan}║${C.reset}  ${C.dim}Miner${C.reset}         : ${C.cyan}${minerAddr}${C.reset}
${C.cyan}║${C.reset}  ${C.dim}Reward${C.reset}        : ${C.green}+${reward}${C.reset} coins
${C.cyan}║${C.reset}  ${C.dim}Transactions${C.reset}  : ${C.yellow}${this.transactions.length}${C.reset}
${C.cyan}║${C.reset}  ${C.dim}Total Fees${C.reset}    : ${C.green}+${this.totalFees}${C.reset} coins
${C.cyan}╠──────────────────────────────────────────────────────╣${C.reset}
${C.cyan}║${C.reset}  ${C.bright}Transactions:${C.reset}
${txDisplay}
${C.cyan}╚══════════════════════════════════════════════════════╝${C.reset}`;
  }
}
exports.Block = Block;
