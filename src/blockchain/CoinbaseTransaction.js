const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

class CoinbaseTransaction {
  constructor(minerAddress, blockHeight) {
    this.from = "COINBASE";
    this.to = minerAddress;
    this.amount = this.calculateReward(blockHeight);
    this.fee = 0;
    this.timestamp = Date.now();
    this.type = "COINBASE";
  }

  calculateReward(blockHeight) {
    // Halving logic: giảm 50% mỗi HALVING_INTERVAL blocks
    // Block 0-99: 50 coins
    // Block 100-199: 25 coins
    // Block 200-299: 12.5 coins
    const halvings = Math.floor(
      blockHeight / BLOCKCHAIN_CONSTANTS.HALVING_INTERVAL
    );
    const reward =
      BLOCKCHAIN_CONSTANTS.INITIAL_MINING_REWARD / Math.pow(2, halvings);
    return reward > 0 ? reward : 0;
  }
  isValid() {
    return true;
  }
}

module.exports = { CoinbaseTransaction };
