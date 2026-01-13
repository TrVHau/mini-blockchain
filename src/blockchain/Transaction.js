const crypto = require("crypto");

class Transaction {
  constructor(from, to, amount, fee = 0) {
    this.from = from;
    this.to = to;
    this.amount = amount;
    this.fee = fee;
    this.timestamp = Date.now();
    this.type = "TRANSFER";
  }

  getTotalCost() {
    return this.amount + this.fee;
  }

  calculateHash() {
    return crypto
      .createHash("sha256")
      .update(
        `${this.from}|${this.to}|${this.amount}|${this.fee}|${this.timestamp}`
      )
      .digest("hex");
  }

  sign(privateKey) {
    const sign = crypto.createSign("SHA256");
    sign.update(this.calculateHash());
    sign.end();
    this.signature = sign.sign(privateKey, "hex");
  }

  isValid(publicKey) {
    if (!this.signature) {
      return false;
    }
    const verify = crypto.createVerify("SHA256");
    verify.update(this.calculateHash());
    verify.end();
    return verify.verify(publicKey, this.signature, "hex");
  }
}

exports.Transaction = Transaction;
