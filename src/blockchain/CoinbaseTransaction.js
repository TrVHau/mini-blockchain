class CoinbaseTransaction {
  constructor(minerAdress, blockHeight) {
    this.from = "COINBASE";
    this.to = minerAdress;
    this.amount = this.calculateReward(blockHeight);
    this.fee = 0;
    this.timestamp = Date.now();
    this.type = "COINBASE";
  }

  calculateReward(blockHeight) {
    // Halving logic: giảm 50% mỗi 100 blocks
    // Block 0-99: 50 coins
    // Block 100-199: 25 coins
    // Block 200-299: 12.5 coins
    const halvingInterval = 100;
    const halvings = Math.floor(blockHeight / halvingInterval);
    const initialReward = 50;
    const reward = initialReward / Math.pow(2, halvings);
    return reward > 0 ? reward : 0;
  }
  isValid() {
    return true;
  }
}
