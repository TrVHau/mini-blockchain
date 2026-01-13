class BalanceTracker {
  constructor() {
    this.balances = new Map();
  }

  // tính toán balance từ blockchain
  updateBalance(blockchain) {
    this.balances.clear();

    blockchain.forEach((block) => {
      // xử lý coinbase transaction
      if (block.coinbaseTx) {
        this.credit(block.coinbaseTx.to, block.coinbaseTx.amount);
      }

      // xử lý regular transactions
      if (block.transactions && block.transactions.length > 0) {
        block.transactions.forEach((tx) => {
          // trừ tiền người gửi
          this.debit(tx.from, tx.amount + (tx.fee || 0));
          // cộng tiền người nhận
          this.credit(tx.to, tx.amount);
        });
      }
    });
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
