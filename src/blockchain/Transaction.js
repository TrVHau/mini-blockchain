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
  }

  getTotalCost() {
    return this.amount + this.fee;
  }

  calculateHash() {
    return crypto
      .createHash("sha256")
      .update(
        `${this.from}|${this.to}|${this.amount}|${this.fee}|${this.timestamp}`,
      )
      .digest("hex");
  }

  sign(privateKey, publicKeyPEM) {
    this.senderPublicKey = publicKeyPEM; // Lưu PEM key để verify
    const sign = crypto.createSign("SHA256");
    sign.update(this.calculateHash());
    sign.end();
    this.signature = sign.sign(privateKey, "hex");
  }

  isValid() {
    if (!this.signature || !this.senderPublicKey) {
      return false;
    }
    const verify = crypto.createVerify("SHA256");
    verify.update(this.calculateHash());
    verify.end();
    return verify.verify(this.senderPublicKey, this.signature, "hex");
  }
}

exports.Transaction = Transaction;
