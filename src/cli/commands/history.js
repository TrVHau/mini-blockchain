const { shortenAddress } = require("../../util/AddressHelper.js");

function historyCommand(vorpal, walletManager, blockchain) {
  vorpal
    .command(
      "history <wallet>",
      "View transaction history for a wallet. Eg: history Alice"
    )
    .alias("h")
    .option(
      "-t ,--type <type>",
      "Filter by transaction type: sent, received, mining_reward"
    )
    .action(function (args, callback) {
      try {
        const address = walletManager.getPublicKey(args.wallet);
        const history = blockchain.getTransactionsHistory(address);

        let filtered = history;

        // Apply filter nếu có
        if (args.options.type) {
          const typeUpper = args.options.type.toUpperCase();
          filtered = history.filter((tx) => {
            if (typeUpper === "SENT") return tx.type === "SENT";
            if (typeUpper === "RECEIVED") return tx.type === "RECEIVED";
            if (typeUpper === "MINING" || typeUpper === "MINING_REWARD") {
              return tx.type === "MINING_REWARD";
            }
            return true;
          });
        }

        if (filtered.length === 0) {
          this.log(`\nNo transactions found for wallet: ${args.wallet}\n`);
        } else {
          this.log(`\n━━━━ Transaction History: ${args.wallet} ━━━━`);
          this.log(`Address: ${shortenAddress(address)}`);

          if (args.options.type) {
            this.log(`Filter: ${args.options.type.toUpperCase()}`);
          }

          this.log(`Total: ${filtered.length} transaction(s)\n`);

          filtered.forEach((tx, i) => {
            const symbol =
              tx.type === "SENT" ? "➜" : tx.type === "RECEIVED" ? "⬅" : "⛏";
            const amountSymbol = tx.type === "SENT" ? "-" : "+";

            this.log(
              `${i + 1}. [Block #${tx.blockIndex}] ${symbol} ${tx.type}`
            );
            this.log(`   From   : ${shortenAddress(tx.from)}`);
            this.log(`   To     : ${shortenAddress(tx.to)}`);
            this.log(`   Amount : ${amountSymbol}${tx.amount} coins`);
            if (tx.fee > 0) this.log(`   Fee    : ${tx.fee} coins`);
            this.log(`   Time   : ${new Date(tx.timestamp).toLocaleString()}`);
            this.log(`   Hash   : ${tx.hash}`);
            this.log("   " + "─".repeat(50));
          });
          this.log("");
        }
      } catch (err) {
        this.log(`Error retrieving history: ${err.message}`);
      }
      callback();
    });
}

module.exports = {
  historyCommand,
};
