const { CoinbaseTransaction } = require("./CoinbaseTransaction.js");
const { shortenAddress } = require("../util/AddressHelper.js");
const MerkleTree = require("../util/MerkleTree.js");
const crypto = require("crypto");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

class Block {
  constructor(index, data, previousHash, minerAddress = null) {
    this.index = index;
    this.data = data;
    this.previousHash = previousHash;
    this.timestamp = Date.now();
    this.nonce = 0;

    this.minerAddress = minerAddress;
    this.coinbaseTx = null;
    this.transactions = [];
    this.totalFees = 0;
    this.merkleRoot = null; // Merkle root của transactions

    this.hash = this.calculateHash();
  }

  /**
   * Tính Merkle Root từ transactions
   */
  calculateMerkleRoot() {
    const txHashes = (this.transactions || []).map((tx) => {
      if (tx.txid) return tx.txid;
      if (tx.calculateHash) return tx.calculateHash();
      return MerkleTree.hash(JSON.stringify(tx));
    });

    // Thêm coinbase tx nếu có
    if (this.coinbaseTx) {
      const coinbaseHash = MerkleTree.hash(
        `${this.coinbaseTx.to}|${this.coinbaseTx.amount}|${this.index}`,
      );
      txHashes.unshift(coinbaseHash);
    }

    if (txHashes.length === 0) {
      return MerkleTree.hash("empty");
    }

    return MerkleTree.calculateRoot(txHashes);
  }

  /**
   * Tính hash của block header
   */
  calculateHash() {
    return crypto
      .createHash("sha256")
      .update(
        `${this.index}|${this.previousHash}|${this.timestamp}|${this.nonce}|${this.merkleRoot || ""}`,
      )
      .digest("hex");
  }

  static genesis() {
    const genesisBlock = new Block(
      0,
      BLOCKCHAIN_CONSTANTS.GENESIS_DATA,
      BLOCKCHAIN_CONSTANTS.GENESIS_PREVIOUS_HASH,
    );
    genesisBlock.timestamp = BLOCKCHAIN_CONSTANTS.GENESIS_TIMESTAMP;
    genesisBlock.merkleRoot = genesisBlock.calculateMerkleRoot();
    genesisBlock.hash = genesisBlock.calculateHash();
    return genesisBlock;
  }

  /**
   * Mine block với Proof of Work
   */
  mineBlock(difficulty, minerAddress) {
    this.totalFees = this.transactions.reduce(
      (sum, tx) => sum + (tx.fee || 0),
      0,
    );

    this.coinbaseTx = new CoinbaseTransaction(
      minerAddress,
      this.index,
      this.totalFees,
    );

    // Tính merkle root sau khi có tất cả transactions
    this.merkleRoot = this.calculateMerkleRoot();

    const target = "0".repeat(difficulty);

    // Mining loop
    while (!this.hash.startsWith(target)) {
      this.nonce++;
      this.hash = this.calculateHash();
    }
  }

  /**
   * Verify một transaction có trong block không (dùng Merkle Proof)
   */
  verifyTransaction(txHash) {
    const txHashes = this.transactions.map(
      (tx) =>
        tx.txid || tx.calculateHash?.() || MerkleTree.hash(JSON.stringify(tx)),
    );

    const index = txHashes.indexOf(txHash);
    if (index === -1) return false;

    const proof = MerkleTree.getProof(txHashes, index);
    return MerkleTree.verifyProof(txHash, proof, this.merkleRoot);
  }

  /**
   * Tính size của block (bytes)
   */
  getSize() {
    return JSON.stringify(this).length;
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
${C.cyan}║${C.reset}  ${C.bright}${C.yellow}⬛ Block #${this.index}${C.reset}                                         ${C.cyan}║${C.reset}
${C.cyan}╠══════════════════════════════════════════════════════╣${C.reset}
${C.cyan}║${C.reset}  ${C.dim}Hash${C.reset}          : ${hashShort}
${C.cyan}║${C.reset}  ${C.dim}Previous${C.reset}      : ${this.previousHash.substring(0, 28)}...
${C.cyan}║${C.reset}  ${C.dim}MerkleRoot${C.reset}    : ${(this.merkleRoot || "").substring(0, 28)}...
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
