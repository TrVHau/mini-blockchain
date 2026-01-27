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

module.exports = {
  mineCommand,
};
