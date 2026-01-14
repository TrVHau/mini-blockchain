// Blockchain Commands: blockchain, block, latest, validate

function blockchainCommand(vorpal, blockchain) {
  vorpal
    .command("blockchain", "See the current state of the blockchain.")
    .alias("bc")
    .action(function (args, callback) {
      const chain = blockchain.get();
      this.log(`\nBlockchain (${chain.length} blocks):\n`);
      chain.forEach((block) => {
        this.log(block.toString());
      });
      callback();
    });
}

function blockCommand(vorpal, blockchain) {
  vorpal
    .command("block <index>", "View details of a specific block. Eg: block 0")
    .alias("b")
    .action(function (args, callback) {
      try {
        const index = parseInt(args.index);

        if (isNaN(index)) {
          this.log("Invalid index! Please provide a number.");
          callback();
          return;
        }

        const chain = blockchain.get();

        if (index >= 0 && index < chain.length) {
          const block = chain[index];
          this.log("\n" + block.toString());
        } else {
          this.log(
            `Block #${index} not found. Chain has ${chain.length} blocks (0-${
              chain.length - 1
            })`
          );
        }
      } catch (err) {
        this.log(`Error getting block: ${err.message}`);
      }
      callback();
    });
}

function latestCommand(vorpal, blockchain) {
  vorpal
    .command("latest", "View the latest block in the chain.")
    .alias("l")
    .action(function (args, callback) {
      const latestBlock = blockchain.getLatestBlock();
      this.log("\n" + latestBlock.toString());
      callback();
    });
}

function validateCommand(vorpal, blockchain) {
  vorpal
    .command("validate", "Validate the integrity of the blockchain.")
    .alias("v")
    .action(function (args, callback) {
      try {
        this.log("Validating blockchain...");
        const isValid = blockchain.isChainValid();
        if (isValid) {
          this.log("Blockchain is valid!");
        } else {
          this.log("Blockchain is NOT valid! Chain has been tampered with.");
        }
      } catch (err) {
        this.log(`Error validating blockchain: ${err.message}`);
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
