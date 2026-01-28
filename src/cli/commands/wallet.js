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

function historyCommand(vorpal, walletManager, blockchain) {
  vorpal
    .command("history <name>", "View transaction history of a wallet")
    .alias("h")
    .action(function (args, callback) {
      try {
        const address = walletManager.getPublicKey(args.name);
        const history = blockchain.getTransactionsHistory(address);

        if (history.length === 0) {
          this.log(UI.info(`No transactions found for ${args.name}`));
        } else {
          this.log(
            `\n${COLORS.cyan}${ICONS.wallet} Transaction History: ${args.name}${COLORS.reset}`,
          );
          this.log(`${COLORS.dim}${"─".repeat(50)}${COLORS.reset}`);

          history.forEach((tx, i) => {
            const date = new Date(tx.timestamp).toLocaleString();
            const typeIcon =
              tx.type === "MINING_REWARD"
                ? ICONS.mining
                : tx.type === "SENT"
                  ? ICONS.send
                  : ICONS.receive;
            const amountColor = tx.type === "SENT" ? COLORS.red : COLORS.green;
            const sign = tx.type === "SENT" ? "-" : "+";

            this.log(
              `  ${COLORS.dim}${i + 1}.${COLORS.reset} ${typeIcon} ${tx.type}`,
            );
            this.log(`     ${COLORS.dim}Date:${COLORS.reset} ${date}`);
            if (tx.type !== "MINING_REWARD") {
              this.log(
                `     ${COLORS.dim}From:${COLORS.reset} ${shortenAddress(tx.from)}`,
              );
              this.log(
                `     ${COLORS.dim}To:${COLORS.reset}   ${shortenAddress(tx.to)}`,
              );
            }
            this.log(
              `     ${COLORS.dim}Amount:${COLORS.reset} ${amountColor}${sign}${tx.amount}${COLORS.reset} coins`,
            );
            if (tx.fee > 0) {
              this.log(
                `     ${COLORS.dim}Fee:${COLORS.reset} ${COLORS.yellow}${tx.fee}${COLORS.reset}`,
              );
            }
            this.log(
              `     ${COLORS.dim}Block:${COLORS.reset} #${tx.blockIndex}`,
            );
            this.log(`${COLORS.dim}${"─".repeat(50)}${COLORS.reset}`);
          });

          // Summary
          const received = history
            .filter((tx) => tx.type !== "SENT")
            .reduce((sum, tx) => sum + tx.amount, 0);
          const sent = history
            .filter((tx) => tx.type === "SENT")
            .reduce((sum, tx) => sum + tx.amount, 0);
          const fees = history
            .filter((tx) => tx.type === "SENT")
            .reduce((sum, tx) => sum + (tx.fee || 0), 0);

          this.log(`\n  ${COLORS.bright}Summary:${COLORS.reset}`);
          this.log(
            `  ${COLORS.green}Total Received: +${received}${COLORS.reset} coins`,
          );
          this.log(`  ${COLORS.red}Total Sent: -${sent}${COLORS.reset} coins`);
          this.log(
            `  ${COLORS.yellow}Total Fees: -${fees}${COLORS.reset} coins\n`,
          );
        }
      } catch (err) {
        this.log(UI.error(`Error: ${err.message}`));
      }
      callback();
    });
}

function exportCommand(vorpal, walletManager) {
  vorpal
    .command("export <name>", "Export wallet private key (KEEP SECRET!)")
    .action(function (args, callback) {
      try {
        const privateKey = walletManager.getPrivateKey(args.name);
        const address = walletManager.getPublicKey(args.name);

        this.log(
          `\n${COLORS.red}⚠ WARNING: KEEP THIS PRIVATE KEY SECRET!${COLORS.reset}`,
        );
        this.log(
          `${COLORS.dim}Anyone with this key can steal your coins.${COLORS.reset}\n`,
        );
        this.log(`${COLORS.cyan}Wallet:${COLORS.reset} ${args.name}`);
        this.log(
          `${COLORS.cyan}Address:${COLORS.reset} ${shortenAddress(address)}\n`,
        );
        this.log(`${COLORS.yellow}Private Key:${COLORS.reset}`);
        this.log(`${COLORS.dim}${privateKey}${COLORS.reset}`);
      } catch (err) {
        this.log(UI.error(`Wallet "${args.name}" not found`));
      }
      callback();
    });
}

function importCommand(vorpal, walletManager) {
  vorpal
    .command("import <name>", "Import wallet from private key")
    .action(function (args, callback) {
      const self = this;

      if (walletManager.hasWallet(args.name)) {
        self.log(UI.error(`Wallet "${args.name}" already exists`));
        callback();
        return;
      }

      self.prompt(
        {
          type: "password",
          name: "privateKey",
          message: "Paste private key (PEM format): ",
        },
        function (result) {
          try {
            const address = walletManager.importWallet(
              args.name,
              result.privateKey,
            );
            self.log(UI.success(`Wallet "${args.name}" imported!`));
            self.log(
              `${COLORS.dim}Address: ${shortenAddress(address)}${COLORS.reset}\n`,
            );
          } catch (err) {
            self.log(UI.error(`Import failed: ${err.message}`));
          }
          callback();
        },
      );
    });
}

module.exports = {
  walletCreateCommand,
  listWalletsCommand,
  balanceCommand,
  addressCommand,
  historyCommand,
  exportCommand,
  importCommand,
};
