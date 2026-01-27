// Transaction Commands: send, mempool
const { Transaction } = require("../../blockchain/Transaction.js");
const { shortenAddress } = require("../../util/AddressHelper.js");
const Validator = require("../../util/Validator.js");
const { UI, COLORS, ICONS } = require("../../util/UI.js");

function sendCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command(
      "send <from> <to> <amount> [fee]",
      "Send coins. <to> can be: wallet name, full address, or address prefix",
    )
    .action(function (args, callback) {
      try {
        // Lấy địa chỉ ví người gửi (phải là wallet local)
        const fromAddress = walletManager.getPublicKey(args.from);
        const privateKey = walletManager.getPrivateKey(args.from);

        // Lấy địa chỉ ví người nhận
        let toAddress;
        let toDisplay = args.to;

        // 1. Thử tìm trong local wallets
        try {
          toAddress = walletManager.getPublicKey(args.to);
        } catch {
          // 2. Nếu là địa chỉ đầy đủ (64 chars hex) -> dùng trực tiếp
          if (args.to.length === 64 && /^[a-f0-9]+$/i.test(args.to)) {
            toAddress = args.to;
            toDisplay = shortenAddress(args.to);
          } else {
            // 3. Tìm địa chỉ match prefix trong blockchain
            const allBalances = blockchain.getAllBalances();
            const matches = Object.keys(allBalances).filter((addr) => {
              // Chỉ match với hash address (64 hex chars)
              if (typeof addr !== "string" || addr.length !== 64) return false;
              return addr.toLowerCase().startsWith(args.to.toLowerCase());
            });

            if (matches.length === 1) {
              toAddress = matches[0];
              toDisplay = shortenAddress(toAddress);
            } else if (matches.length > 1) {
              throw new Error(
                `Multiple addresses match "${args.to}". Be more specific.`,
              );
            } else {
              throw new Error(
                `Address not found: "${args.to}". Use full address or wallet name.`,
              );
            }
          }
        }

        // tạo transaction
        const amount = Validator.validateAmount(parseFloat(args.amount));
        const fee = args.fee
          ? Validator.validateAmount(parseFloat(args.fee))
          : 0;
        const transaction = new Transaction(
          fromAddress,
          toAddress,
          amount,
          fee,
        );

        // ký transaction (cần cả private key và PEM public key)
        const senderPEM = walletManager.getPEMPublicKey(args.from);
        transaction.sign(privateKey, senderPEM);

        // thêm vào mempool
        blockchain.addTransaction(transaction);

        // broadcast transaction đến các peer
        p2p.broadcastTransaction(transaction);

        const content = [
          UI.keyValue("From", `${COLORS.cyan}${args.from}${COLORS.reset}`),
          UI.keyValue("To", `${COLORS.cyan}${toDisplay}${COLORS.reset}`),
          UI.divider("─", 35),
          UI.keyValue(
            "Amount",
            `${COLORS.yellow}${amount}${COLORS.reset} coins`,
          ),
          UI.keyValue("Fee", `${COLORS.dim}${fee}${COLORS.reset} coins`),
          UI.keyValue(
            "Total",
            `${COLORS.red}-${amount + fee}${COLORS.reset} coins`,
          ),
        ].join("\n");

        this.log(
          "\n" + UI.box(content, `${ICONS.send} Transaction Created`, 40),
        );
        this.log(UI.success(`Broadcasted to ${p2p.peers.length} peer(s)`));
        this.log(UI.info(`Wait for it to be mined into a block.\n`));
      } catch (err) {
        this.log(UI.error(`Error: ${err.message}`));
      }
      callback();
    });
}

function mempoolCommand(vorpal, blockchain) {
  vorpal
    .command("mempool", "View pending transactions in mempool.")
    .alias("mp")
    .action(function (args, callback) {
      if (blockchain.mempool.length === 0) {
        this.log(UI.info("Mempool is empty."));
      } else {
        const content = blockchain.mempool
          .map((tx, i) => {
            return [
              `${COLORS.cyan}${i + 1}.${COLORS.reset} ${ICONS.send} ${tx.type}`,
              `   From   : ${shortenAddress(tx.from)}`,
              `   To     : ${shortenAddress(tx.to)}`,
              `   Amount : ${COLORS.yellow}${tx.amount}${COLORS.reset} coins`,
              `   Fee    : ${COLORS.dim}${tx.fee}${COLORS.reset} coins`,
            ].join("\n");
          })
          .join("\n" + UI.divider("─", 45) + "\n");

        const totalFees = blockchain.mempool.reduce(
          (sum, tx) => sum + (tx.fee || 0),
          0,
        );

        const footer = `\n${ICONS.coin} Total Fees Available: ${COLORS.green}${totalFees}${COLORS.reset} coins`;

        this.log(
          "\n" +
            UI.box(
              content + footer,
              `${ICONS.pending} Mempool (${blockchain.mempool.length} tx)`,
              50,
            ),
        );
      }
      callback();
    });
}

module.exports = {
  sendCommand,
  mempoolCommand,
};
