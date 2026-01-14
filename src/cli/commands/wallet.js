// Wallet Commands: wallet-create, wallets, balance
const { shortenAddress } = require("../../util/AddressHelper.js");

function walletCreateCommand(vorpal, walletManager) {
  vorpal
    .command(
      "wallet-create <name>",
      "Create a new wallet. Eg: wallet-create Alice"
    )
    .alias("wc")
    .action(function (args, callback) {
      try {
        const publicKey = walletManager.createWallet(args.name);
        this.log(`\nWallet created: ${args.name}`);
        this.log(`Address: ${shortenAddress(publicKey)}\n`);
      } catch (err) {
        this.log(`Error creating wallet: ${err.message}`);
      }
      callback();
    });
}

function listWalletsCommand(vorpal, walletManager, blockchain) {
  vorpal
    .command(
      "wallets [all]",
      "List all wallets. Use 'wallets all' to see all addresses with balance."
    )
    .alias("wl")
    .action(function (args, callback) {
      try {
        if (args.all === "all") {
          // Show all addresses with balance (not just wallets)
          const balances = blockchain.getAllBalances();
          const entries = Object.entries(balances);
          if (entries.length === 0) {
            this.log("No addresses with balance.");
          } else {
            this.log(`\nAll Addresses with Balance (${entries.length}):\n`);
            entries.forEach(([address, balance], i) => {
              this.log(`${i + 1}. ${shortenAddress(address)}`);
              this.log(`   Balance: ${balance} coins`);
              this.log("-----------------------");
            });
          }
        } else {
          // Show only managed wallets
          const wallets = walletManager.listWallets();
          if (wallets.length === 0) {
            this.log("No wallets found. Create one with: wallet-create <name>");
          } else {
            this.log(`\nManaged Wallets (${wallets.length}):\n`);
            wallets.forEach((name, i) => {
              const address = walletManager.getPublicKey(name);
              const balance = blockchain.getBalance(address);
              this.log(`${i + 1}. ${name}`);
              this.log(`   Address: ${shortenAddress(address)}`);
              this.log(`   Balance: ${balance} coins`);
              this.log("-----------------------");
            });
          }
        }
      } catch (err) {
        this.log(`Error listing wallets: ${err.message}`);
      }
      callback();
    });
}

function balanceCommand(vorpal, walletManager, blockchain) {
  vorpal
    .command("balance <name>", "Check balance of a wallet. Eg: balance Alice")
    .alias("bal")
    .action(function (args, callback) {
      try {
        const address = walletManager.getPublicKey(args.name);
        const balance = blockchain.getBalance(address);
        this.log(`\nWallet: ${args.name}`);
        this.log(`Address: ${shortenAddress(address)}`);
        this.log(`Balance: ${balance} coins\n`);
      } catch (err) {
        this.log(`Error checking balance: ${err.message}`);
      }
      callback();
    });
}

module.exports = {
  walletCreateCommand,
  listWalletsCommand,
  balanceCommand,
};
