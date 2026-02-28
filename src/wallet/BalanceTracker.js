class BalanceTracker {
  constructor() {
    this.balances = new Map();
  }

  // Rebuild toàn bộ balance từ chain (dùng khi receiveChain)
  updateBalance(blockchain) {
    this.balances.clear();
    blockchain.forEach((block) => this._processBlock(block));
  }

  // Incremental update từ 1 block mới (O(1) thay vì O(n))
  processBlock(block) {
    this._processBlock(block);
  }

  _processBlock(block) {
    // Coinbase reward (đã bao gồm fees)
    if (block.coinbaseTx) {
      this.credit(block.coinbaseTx.to, block.coinbaseTx.amount);
    }

    // Regular transactions
    if (block.transactions && block.transactions.length > 0) {
      block.transactions.forEach((tx) => {
        this.debit(tx.from, tx.amount + (tx.fee || 0));
        this.credit(tx.to, tx.amount);
      });
    }
  }

  credit(address, amount) {
    const currentBalance = this.balances.get(address) || 0;
    this.balances.set(address, currentBalance + amount);
  }

  debit(address, amount) {
    const currentBalance = this.balances.get(address) || 0;
    this.balances.set(address, currentBalance - amount);
  }

  getBalance(address) {
    return this.balances.get(address) || 0;
  }

  hasBalance(address, amount) {
    const balance = this.getBalance(address);
    return balance >= amount;
  }

  getAllBalances() {
    const result = {};
    for (const [address, balance] of this.balances.entries()) {
      result[address] = balance;
    }
    return result;
  }
}

module.exports = { BalanceTracker };
