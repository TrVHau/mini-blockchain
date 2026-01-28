// Blockchain Commands
const { UI, COLORS, ICONS } = require("../../util/UI.js");

function blockchainCommand(vorpal, blockchain) {
  vorpal
    .command("blockchain", "View entire blockchain")
    .alias("bc")
    .action(function (args, callback) {
      const chain = blockchain.get();
      this.log(
        `\n${COLORS.cyan}${ICONS.chain} Blockchain (${chain.length} blocks)${COLORS.reset}\n`,
      );
      chain.forEach((block) => this.log(block.toString()));
      callback();
    });
}

function blockCommand(vorpal, blockchain) {
  vorpal
    .command("block <index>", "View specific block")
    .alias("b")
    .action(function (args, callback) {
      const index = parseInt(args.index);
      const chain = blockchain.get();

      if (isNaN(index) || index < 0 || index >= chain.length) {
        this.log(UI.error(`Block not found. Valid: 0-${chain.length - 1}`));
      } else {
        this.log("\n" + chain[index].toString());
      }
      callback();
    });
}

function latestCommand(vorpal, blockchain) {
  vorpal
    .command("latest", "View latest block")
    .alias("l")
    .action(function (args, callback) {
      this.log("\n" + blockchain.getLatestBlock().toString());
      callback();
    });
}

function validateCommand(vorpal, blockchain) {
  vorpal
    .command("validate", "Validate blockchain")
    .alias("v")
    .action(function (args, callback) {
      if (blockchain.isChainValid()) {
        this.log(UI.success("Blockchain is valid"));
      } else {
        this.log(UI.error("Blockchain is invalid!"));
      }
      callback();
    });
}

function statsCommand(vorpal, blockchain) {
  vorpal
    .command("stats", "View blockchain statistics")
    .action(function (args, callback) {
      const stats = blockchain.getStats();
      const fee = blockchain.estimateFee();
      const reward = blockchain.getBlockReward();

      this.log(
        `\n${COLORS.cyan}${ICONS.chain} Blockchain Statistics${COLORS.reset}`,
      );
      this.log(`${COLORS.dim}─────────────────────────────${COLORS.reset}`);
      this.log(
        `  Height:          ${COLORS.yellow}${stats.totalBlocks}${COLORS.reset} blocks`,
      );
      this.log(
        `  Difficulty:      ${COLORS.yellow}${stats.difficulty}${COLORS.reset}`,
      );
      this.log(
        `  Pending TX:      ${COLORS.yellow}${stats.mempoolSize}${COLORS.reset}`,
      );
      this.log(
        `  Total TX:        ${COLORS.yellow}${stats.totalTransactions}${COLORS.reset}`,
      );
      this.log(
        `  Total Coins:     ${COLORS.green}${stats.totalCoins}${COLORS.reset} coins`,
      );
      this.log(
        `  Block Reward:    ${COLORS.green}${reward}${COLORS.reset} coins`,
      );
      this.log(`  Estimated Fee:   ${COLORS.green}${fee}${COLORS.reset} coins`);
      this.log(
        `  Avg Block Time:  ${COLORS.yellow}${stats.avgBlockTime}${COLORS.reset}s`,
      );
      this.log(
        `  Valid:           ${blockchain.isChainValid() ? COLORS.green + "✓" : COLORS.red + "✗"}${COLORS.reset}`,
      );
      this.log("");
      callback();
    });
}

function txCommand(vorpal, blockchain) {
  vorpal
    .command("tx <txid>", "View transaction details")
    .action(function (args, callback) {
      const txid = args.txid;
      const result = blockchain.getTransaction(txid);

      if (!result) {
        this.log(UI.error(`Transaction not found: ${txid}`));
      } else {
        const { transaction, blockIndex, confirmations } = result;
        const confirmed = blockchain.isConfirmed(txid);

        this.log(
          `\n${COLORS.cyan}${ICONS.tx} Transaction Details${COLORS.reset}`,
        );
        this.log(`${COLORS.dim}─────────────────────────────${COLORS.reset}`);
        this.log(
          `  TxID:        ${COLORS.yellow}${transaction.txid}${COLORS.reset}`,
        );
        this.log(`  Block:       ${COLORS.cyan}#${blockIndex}${COLORS.reset}`);
        this.log(`  From:        ${transaction.from.substring(0, 16)}...`);
        this.log(`  To:          ${transaction.to.substring(0, 16)}...`);
        this.log(
          `  Amount:      ${COLORS.green}${transaction.amount}${COLORS.reset} coins`,
        );
        this.log(
          `  Fee:         ${COLORS.yellow}${transaction.fee || 0}${COLORS.reset} coins`,
        );
        this.log(
          `  Confirmations: ${confirmations >= 6 ? COLORS.green : COLORS.yellow}${confirmations}${COLORS.reset}`,
        );
        this.log(
          `  Status:      ${confirmed ? COLORS.green + "Confirmed ✓" : COLORS.yellow + "Pending"}${COLORS.reset}`,
        );
        this.log("");
      }
      callback();
    });
}

function mempoolCommand(vorpal, blockchain) {
  vorpal
    .command("mempool", "View pending transactions")
    .alias("mp")
    .action(function (args, callback) {
      const pending = blockchain.getPendingTransactions();

      if (pending.length === 0) {
        this.log(UI.info("Mempool is empty"));
      } else {
        this.log(
          `\n${COLORS.cyan}${ICONS.tx} Mempool (${pending.length} pending)${COLORS.reset}`,
        );
        this.log(`${COLORS.dim}─────────────────────────────${COLORS.reset}`);
        pending.forEach((tx, i) => {
          const from = tx.from.substring(0, 12);
          const to = tx.to.substring(0, 12);
          this.log(
            `  ${i + 1}. ${from}... → ${to}... | ${COLORS.green}${tx.amount}${COLORS.reset} coins | fee: ${tx.fee || 0}`,
          );
        });
        this.log("");
      }
      callback();
    });
}

function feeCommand(vorpal, blockchain) {
  vorpal
    .command("fee", "Estimate transaction fee")
    .action(function (args, callback) {
      const fee = blockchain.estimateFee();
      this.log(
        UI.info(`Estimated fee: ${COLORS.green}${fee}${COLORS.reset} coins`),
      );
      callback();
    });
}

module.exports = {
  blockchainCommand,
  blockCommand,
  latestCommand,
  validateCommand,
  statsCommand,
  txCommand,
  mempoolCommand,
  feeCommand,
};
