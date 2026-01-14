// Transaction Commands: send, mempool
const { Transaction } = require("../../blockchain/Transaction.js");
const { shortenAddress } = require("../../util/AddressHelper.js");
const Validator = require("../../util/Validator.js");

function sendCommand(vorpal, walletManager, blockchain, p2p) {
  vorpal
    .command(
      "send <from> <to> <amount> [fee]",
      "Send coins from one wallet to another. Eg: send Alice Bob 10"
    )
    .action(function (args, callback) {
      try {
        // Lấy địa chỉ ví người gửi
        const fromAddress = walletManager.getPublicKey(args.from);
        const privateKey = walletManager.getPrivateKey(args.from);

        // Lấy địa chỉ ví người nhận
        let toAddress;
        try {
          toAddress = walletManager.getPublicKey(args.to);
        } catch {
          toAddress = args.to; // nếu không phải ví trong hệ thống thì dùng địa chỉ thẳng
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
          fee
        );

        // ký transaction
        transaction.sign(privateKey);

        // thêm vào mempool
        blockchain.addTransaction(transaction, fromAddress);

        // broadcast transaction đến các peer
        p2p.broadcastTransaction(transaction);
        this.log(`\nTransaction created and broadcasted successfully!`);
        this.log(`From: ${args.from}`);
        this.log(`To  : ${args.to}`);
        this.log(`Amount: ${amount} coins`);
        this.log(`Fee   : ${fee} coins`);
        this.log(`Wait for it to be mined into a block.\n`);
      } catch (err) {
        this.log(`Error sending transaction: ${err.message}`);
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
        this.log("Mempool is empty.");
      } else {
        this.log(`\nMempool (${blockchain.mempool.length} transactions):\n`);
        blockchain.mempool.forEach((tx, i) => {
          this.log(`${i + 1}. Transaction`);
          this.log(`   From   : ${shortenAddress(tx.from)}`);
          this.log(`   To     : ${shortenAddress(tx.to)}`);
          this.log(`   Amount : ${tx.amount} coins`);
          this.log(`   Fee    : ${tx.fee} coins`);
          this.log(`   Type   : ${tx.type}`);
          this.log("-----------------------");
        });
        const totalFees = blockchain.mempool.reduce(
          (sum, tx) => sum + (tx.fee || 0),
          0
        );
        this.log(`\nTotal Fees Available: ${totalFees} coins\n`);
      }
      callback();
    });
}

module.exports = {
  sendCommand,
  mempoolCommand,
};
