// Wallet Commands: wallet-create, wallets, balance
const { shortenAddress } = require("../../util/AddressHelper.js");
const Validator = require("../../util/Validator.js");
const { UI, COLORS, ICONS } = require("../../util/UI.js");

function walletCreateCommand(vorpal, walletManager) {
  vorpal
    .command(
      "wallet-create <name>",
      "Create a new wallet. Eg: wallet-create Alice",
    )
    .alias("wc")
    .action(function (args, callback) {
      try {
        const name = Validator.validateWalletName(args.name);
        const publicKey = walletManager.createWallet(name);

        const content = [
          UI.keyValue("Name", `${COLORS.cyan}${args.name}${COLORS.reset}`),
          UI.keyValue(
            "Address",
            `${COLORS.yellow}${shortenAddress(publicKey)}${COLORS.reset}`,
          ),
        ].join("\n");

        this.log("\n" + UI.box(content, `${ICONS.wallet} Wallet Created`, 50));
        this.log(UI.success("Wallet created successfully!\n"));
      } catch (err) {
        this.log(UI.error(`Error creating wallet: ${err.message}`));
      }
      callback();
    });
}

function listWalletsCommand(vorpal, walletManager, blockchain) {
  vorpal
    .command(
      "wallets [all]",
      "List all wallets. Use 'wallets all' to see all addresses with balance.",
    )
    .alias("wl")
    .action(function (args, callback) {
      try {
        if (args.all === "all") {
          // Show all addresses with balance (not just wallets)
          const balances = blockchain.getAllBalances();
          const entries = Object.entries(balances).filter(([_, b]) => b > 0);
          if (entries.length === 0) {
            this.log(UI.warning("No addresses with balance."));
          } else {
            const content = entries
              .map(([address, balance], i) => {
                return `${COLORS.cyan}${i + 1}.${COLORS.reset} ${shortenAddress(address)}\n   ${ICONS.coin} ${COLORS.yellow}${balance}${COLORS.reset} coins`;
              })
              .join("\n" + UI.divider("─", 45) + "\n");

            this.log(
              "\n" +
                UI.box(
                  content,
                  `${ICONS.wallet} All Addresses (${entries.length})`,
                  50,
                ),
            );
          }
        } else {
          // Show only managed wallets
          const wallets = walletManager.listWallets();
          if (wallets.length === 0) {
            this.log(
              UI.warning(
                "No wallets found. Create one with: wallet-create <name>",
              ),
            );
          } else {
            const content = wallets
              .map((name, i) => {
                const address = walletManager.getPublicKey(name);
                const balance = blockchain.getBalance(address);
                return `${COLORS.cyan}${i + 1}.${COLORS.reset} ${COLORS.bright}${name}${COLORS.reset}\n   ${ICONS.arrow} ${shortenAddress(address)}\n   ${ICONS.coin} ${COLORS.yellow}${balance}${COLORS.reset} coins`;
              })
              .join("\n" + UI.divider("─", 45) + "\n");

            this.log(
              "\n" +
                UI.box(
                  content,
                  `${ICONS.wallet} Managed Wallets (${wallets.length})`,
                  50,
                ),
            );
          }
        }
      } catch (err) {
        this.log(UI.error(`Error listing wallets: ${err.message}`));
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

        const content = [
          UI.keyValue("Wallet", `${COLORS.bright}${args.name}${COLORS.reset}`),
          UI.keyValue(
            "Address",
            `${COLORS.dim}${shortenAddress(address)}${COLORS.reset}`,
          ),
          UI.divider("─", 35),
          UI.keyValue(
            "Balance",
            `${COLORS.green}${ICONS.coin} ${balance}${COLORS.reset} coins`,
          ),
        ].join("\n");

        this.log("\n" + UI.box(content, `${ICONS.wallet} Balance`, 40));
      } catch (err) {
        this.log(UI.error(`Error checking balance: ${err.message}`));
      }
      callback();
    });
}

function addressCommand(vorpal, walletManager) {
  vorpal
    .command("address <name>", "Show full address of wallet (for sharing)")
    .alias("addr")
    .action(function (args, callback) {
      try {
        const address = walletManager.getPublicKey(args.name);
        this.log(`\n${COLORS.cyan}${args.name}${COLORS.reset} address:\n`);
        this.log(`${COLORS.yellow}${address}${COLORS.reset}\n`);
        this.log(
          `${COLORS.dim}(Copy and share this address to receive coins)${COLORS.reset}\n`,
        );
      } catch (err) {
        this.log(UI.error(`Wallet "${args.name}" not found`));
      }
      callback();
    });
}

module.exports = {
  walletCreateCommand,
  listWalletsCommand,
  balanceCommand,
  addressCommand,
};
