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

module.exports = {
  blockchainCommand,
  blockCommand,
  latestCommand,
  validateCommand,
};
