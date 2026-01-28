// Mining Commands
const { UI, COLORS, ICONS } = require("../../util/UI.js");
const { shortenAddress } = require("../../util/AddressHelper.js");

function mineCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command(
      "mine <wallet>",
      "Mine block. <wallet> can be wallet name or address",
    )
    .alias("m")
    .action(function (args, callback) {
      try {
        let minerAddress;
        let displayName = args.wallet;

        // 1. Thử tìm trong local wallets
        try {
          minerAddress = walletManager.getPublicKey(args.wallet);
        } catch {
          // 2. Nếu là địa chỉ (64 hex chars) -> dùng trực tiếp
          if (args.wallet.length === 64 && /^[a-f0-9]+$/i.test(args.wallet)) {
            minerAddress = args.wallet;
            displayName = shortenAddress(args.wallet);
          } else {
            // 3. Tìm prefix trong blockchain
            const allBalances = blockchain.getAllBalances();
            const matches = Object.keys(allBalances).filter((addr) => {
              if (typeof addr !== "string" || addr.length !== 64) return false;
              return addr.toLowerCase().startsWith(args.wallet.toLowerCase());
            });

            if (matches.length === 1) {
              minerAddress = matches[0];
              displayName = shortenAddress(minerAddress);
            } else if (matches.length > 1) {
              throw new Error(`Multiple addresses match "${args.wallet}"`);
            } else {
              throw new Error(`Wallet/address not found: "${args.wallet}"`);
            }
          }
        }

        this.log(UI.info(`Mining block for ${displayName}...`));

        const newBlock = blockchain.mineBlock(minerAddress);

        if (newBlock) {
          const content = [
            UI.keyValue(
              "Block",
              `#${COLORS.cyan}${newBlock.index}${COLORS.reset}`,
            ),
            UI.keyValue(
              "Transactions",
              `${COLORS.yellow}${newBlock.transactions.length}${COLORS.reset}`,
            ),
            UI.keyValue(
              "Nonce",
              `${COLORS.dim}${newBlock.nonce}${COLORS.reset}`,
            ),
            UI.keyValue(
              "Hash",
              `${COLORS.dim}${newBlock.hash.slice(0, 16)}...${COLORS.reset}`,
            ),
          ].join("\n");

          this.log("\n" + UI.box(content, `${ICONS.mining} Block Mined`, 45));

          const reward = newBlock.coinbaseTx ? newBlock.coinbaseTx.amount : 0;
          this.log(UI.success(`Reward: ${reward} coins`));

          // Broadcast block
          p2p.broadcastNewBlock(newBlock);
          this.log(UI.info(`Broadcasted to ${p2p.peers.length} peer(s)\n`));
        } else {
          this.log(UI.error("Mining failed"));
        }
      } catch (err) {
        this.log(UI.error(`Error: ${err.message}`));
      }
      callback();
    });
}

// Store auto-mine interval globally
let autoMineInterval = null;

function autoMineCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command(
      "automine <wallet> [interval]",
      "Auto-mine when mempool has transactions. Interval in seconds (default: 10)",
    )
    .alias("am")
    .action(function (args, callback) {
      try {
        // Stop existing auto-mine
        if (autoMineInterval) {
          clearInterval(autoMineInterval);
          autoMineInterval = null;
          this.log(UI.info("Auto-mine stopped"));
          callback();
          return;
        }

        let minerAddress;
        try {
          minerAddress = walletManager.getPublicKey(args.wallet);
        } catch {
          if (args.wallet.length === 64 && /^[a-f0-9]+$/i.test(args.wallet)) {
            minerAddress = args.wallet;
          } else {
            throw new Error(`Wallet not found: ${args.wallet}`);
          }
        }

        const intervalSec = parseInt(args.interval) || 10;
        const displayName = shortenAddress(minerAddress);

        this.log(
          UI.success(
            `Auto-mine started for ${displayName} (every ${intervalSec}s)`,
          ),
        );
        this.log(UI.info("Run 'automine' again to stop\n"));

        const self = this;

        autoMineInterval = setInterval(() => {
          if (blockchain.mempool.length > 0) {
            self.log(
              UI.info(
                `Auto-mining ${blockchain.mempool.length} pending tx(s)...`,
              ),
            );
            try {
              const newBlock = blockchain.mineBlock(minerAddress);
              if (newBlock) {
                const reward = newBlock.coinbaseTx
                  ? newBlock.coinbaseTx.amount
                  : 0;
                self.log(
                  UI.success(
                    `Block #${newBlock.index} mined! Reward: ${reward} coins`,
                  ),
                );
                p2p.broadcastNewBlock(newBlock);
              }
            } catch (err) {
              self.log(UI.error(`Auto-mine error: ${err.message}`));
            }
          }
        }, intervalSec * 1000);
      } catch (err) {
        this.log(UI.error(`Error: ${err.message}`));
      }
      callback();
    });
}

function stopAutoMineCommand(vorpal) {
  vorpal
    .command("stopautomine", "Stop auto-mining")
    .alias("sam")
    .action(function (args, callback) {
      if (autoMineInterval) {
        clearInterval(autoMineInterval);
        autoMineInterval = null;
        this.log(UI.success("Auto-mine stopped"));
      } else {
        this.log(UI.info("Auto-mine is not running"));
      }
      callback();
    });
}

module.exports = {
  mineCommand,
  autoMineCommand,
  stopAutoMineCommand,
};
