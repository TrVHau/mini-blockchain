const crypto = require("crypto");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

class Transaction {
  constructor(
    from,
    to,
    amount,
    fee = BLOCKCHAIN_CONSTANTS.DEFAULT_TRANSACTION_FEE,
  ) {
    this.from = from; // Hash address
    this.to = to; // Hash address
    this.amount = amount;
    this.fee = fee;
    this.timestamp = Date.now();
    this.type = "TRANSFER";
    this.senderPublicKey = null; // PEM key for signature verification
    this.signature = null;
    this.txid = null; // Transaction ID - set after signing
  }

  getTotalCost() {
    return this.amount + this.fee;
  }

  /**
   * Tính hash của transaction data (không bao gồm signature)
   */
  calculateHash() {
    return crypto
      .createHash("sha256")
      .update(
        `${this.from}|${this.to}|${this.amount}|${this.fee}|${this.timestamp}`,
      )
      .digest("hex");
  }

  /**
   * Tính Transaction ID (hash bao gồm cả signature)
   */
  calculateTxid() {
    const data = `${this.from}|${this.to}|${this.amount}|${this.fee}|${this.timestamp}|${this.signature}`;
    return crypto.createHash("sha256").update(data).digest("hex");
  }

  /**
   * Ký transaction và tạo txid
   */
  sign(privateKey, publicKeyPEM) {
    this.senderPublicKey = publicKeyPEM;
    const sign = crypto.createSign("SHA256");
    sign.update(this.calculateHash());
    sign.end();
    this.signature = sign.sign(privateKey, "hex");
    this.txid = this.calculateTxid(); // Tạo txid sau khi ký
  }

  /**
   * Verify signature
   */
  isValid() {
    if (!this.signature || !this.senderPublicKey) {
      return false;
    }
    const verify = crypto.createVerify("SHA256");
    verify.update(this.calculateHash());
    verify.end();
    return verify.verify(this.senderPublicKey, this.signature, "hex");
  }

  /**
   * Serialize transaction để tính size
   */
  getSize() {
    return JSON.stringify(this).length;
  }
}

exports.Transaction = Transaction;
