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
    .command("block <query>", "View block by index or hash (prefix)")
    .alias("b")
    .action(function (args, callback) {
      const chain = blockchain.get();
      const query = args.query;

      // Try as index first
      const index = parseInt(query);
      if (!isNaN(index) && index >= 0 && index < chain.length) {
        this.log("\n" + chain[index].toString());
        callback();
        return;
      }

      // Try as hash prefix
      if (/^[a-f0-9]+$/i.test(query)) {
        const matches = chain.filter((block) =>
          block.hash.toLowerCase().startsWith(query.toLowerCase()),
        );

        if (matches.length === 1) {
          this.log("\n" + matches[0].toString());
        } else if (matches.length > 1) {
          this.log(UI.warning(`Multiple blocks match "${query}":`));
          matches.forEach((block) => {
            this.log(`  #${block.index}: ${block.hash.substring(0, 20)}...`);
          });
        } else {
          this.log(UI.error(`Block not found: "${query}"`));
        }
      } else {
        this.log(UI.error(`Invalid query. Use block index or hash prefix.`));
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
    .command("tx <txid>", "View transaction by txid or prefix")
    .action(function (args, callback) {
      const query = args.txid;

      // Tìm exact match trước
      let result = blockchain.getTransaction(query);

      // Nếu không tìm thấy, thử tìm theo prefix
      if (!result && /^[a-f0-9]+$/i.test(query)) {
        const chain = blockchain.get();
        for (const block of chain) {
          if (block.transactions) {
            const match = block.transactions.find(
              (tx) =>
                tx.txid &&
                tx.txid.toLowerCase().startsWith(query.toLowerCase()),
            );
            if (match) {
              result = blockchain.getTransaction(match.txid);
              break;
            }
          }
        }
      }

      if (!result) {
        this.log(UI.error(`Transaction not found: ${query}`));
      } else {
        const { transaction, blockIndex, confirmations } = result;
        const confirmed = blockchain.isConfirmed(transaction.txid);

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
