// Utility Commands: export, import, clear

function exportCommand(vorpal, blockchain) {
  vorpal
    .command(
      "export <filename>",
      "Export blockchain to JSON file. Eg: export chain.json"
    )
    .action(function (args, callback) {
      const fs = require("fs");
      const data = {
        chain: blockchain.get(),
        difficulty: blockchain.difficulty,
        exportDate: new Date().toISOString(),
      };

      try {
        fs.writeFileSync(args.filename, JSON.stringify(data, null, 2));
        this.log(`Blockchain exported to ${args.filename}`);
      } catch (err) {
        this.log(`Error exporting: ${err.message}`);
      }
      callback();
    });
}

function importCommand(vorpal, blockchain) {
  vorpal
    .command(
      "import <filename>",
      "Import blockchain from JSON file. Eg: import chain.json"
    )
    .action(function (args, callback) {
      const fs = require("fs");

      try {
        const data = JSON.parse(fs.readFileSync(args.filename, "utf8"));
        if (data.chain && Array.isArray(data.chain)) {
          blockchain.receiveChain(data.chain);
          if (data.difficulty) {
            blockchain.difficulty = data.difficulty;
          }
          this.log(` Blockchain imported from ${args.filename}`);
          this.log(`   Loaded ${data.chain.length} blocks`);
        } else {
          this.log(" Invalid blockchain file format");
        }
      } catch (err) {
        this.log(` Error importing: ${err.message}`);
      }
      callback();
    });
}

function clearCommand(vorpal) {
  vorpal
    .command("clear", "Clear the terminal screen.")
    .action(function (args, callback) {
      console.clear();
      this.log("Welcome to Blockchain CLI!");
      callback();
    });
}

module.exports = {
  exportCommand,
  importCommand,
  clearCommand,
};
