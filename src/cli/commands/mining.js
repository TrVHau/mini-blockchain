// Mining Commands: mine, mine-mempool, difficulty
const { shortenAddress } = require("../../util/AddressHelper.js");
const BLOCKCHAIN_CONSTANTS = require("../../config/constants.js");

function mineCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command("mine <wallet> [data]", "Mine a new block. Eg: mine hello!")
    .alias("m")
    .action(function (args, callback) {
      try {
        // lấy miner address
        const minerAddress = walletManager.getPublicKey(args.wallet);
        const data = args.data || "";

        // đào block mới
        this.log(`\nMining new block for wallet: ${args.wallet}...`);
        const block = blockchain.mineBlock(minerAddress, data);

        // broadcast block mới đến các peer
        p2p.broadcastNewBlock(block);
        const reward = blockchain.getMiningReward();

        this.log(`\nBlock mined successfully!`);
        this.log(`Block #${block.index}`);
        this.log(`Hash: ${block.hash}`);
        this.log(`Miner: ${shortenAddress(minerAddress)}`);
        this.log(`Reward: ${reward} coins`);
        this.log(`Transactions: ${block.transactions.length}`);
        this.log(`Total Fees: ${block.totalFees} coins\n`);
      } catch (err) {
        this.log(`Error mining block: ${err.message}`);
      }
      callback();
    });
}

function mineMempoolCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command(
      "mine-mempool <wallet>",
      "Mine all pending transactions in mempool. Eg: mine-mempool Alice"
    )
    .alias("mm")
    .action(function (args, callback) {
      try {
        if (blockchain.mempool.length === 0) {
          this.log("Mempool is empty, nothing to mine.");
        } else {
          // Lấy miner address
          const minerAddress = walletManager.getPublicKey(args.wallet);

          this.log(
            `Mining ${blockchain.mempool.length} transactions for wallet: ${args.wallet}...`
          );
          const block = blockchain.mineMempool(minerAddress);

          // Broadcast block mới đến các peer
          p2p.broadcastNewBlock(block);

          const reward = blockchain.getMiningReward();
          this.log(`\nBlock mined and broadcasted successfully!`);
          this.log(`Block #${block.index}`);
          this.log(`Miner: ${args.wallet}`);
          this.log(`Reward: ${reward} coins`);
          this.log(`Transactions: ${block.transactions.length}`);
          this.log(`Total Fees: ${block.totalFees} coins\n`);
        }
      } catch (err) {
        this.log(`Error mining mempool: ${err.message}`);
      }
      callback();
    });
}

function difficultyCommand(vorpal, blockchain) {
  vorpal
    .command(
      "difficulty <level>",
      "Set mining difficulty (1-6). Eg: difficulty 3"
    )
    .action(function (args, callback) {
      const level = parseInt(args.level);
      if (
        level >= BLOCKCHAIN_CONSTANTS.MIN_DIFFICULTY &&
        level <= BLOCKCHAIN_CONSTANTS.MAX_DIFFICULTY
      ) {
        blockchain.difficulty = level;
        this.log(` Mining difficulty set to ${level}`);
        this.log(`   Blocks must start with ${"0".repeat(level)}...`);
      } else {
        this.log(
          ` Difficulty must be between ${BLOCKCHAIN_CONSTANTS.MIN_DIFFICULTY} and ${BLOCKCHAIN_CONSTANTS.MAX_DIFFICULTY}`
        );
      }
      callback();
    });
}

module.exports = {
  mineCommand,
  mineMempoolCommand,
  difficultyCommand,
};
