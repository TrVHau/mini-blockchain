const Vorpal = require("vorpal");
const { P2P } = require("../p2p/P2P.js");
const { BlockChain } = require("../blockchain/BlockChain.js");
const { WalletManager } = require("../wallet/WalletManager.js");

// Import command modules
const networkCommands = require("./commands/network.js");
const walletCommands = require("./commands/wallet.js");
const transactionCommands = require("./commands/transaction.js");
const miningCommands = require("./commands/mining.js");
const blockchainCommands = require("./commands/blockchain.js");
const utilityCommands = require("./commands/utility.js");

// Khởi tạo các thành phần chính
const walletManager = new WalletManager();
const blockchain = new BlockChain();
const p2p = new P2P(blockchain);

// Khởi tạo Vorpal CLI
function cli() {
  const vorpal = Vorpal();

  // Welcome message
  vorpal.log("Welcome to Blockchain CLI!");
  vorpal.exec("help");

  // Load Network Commands
  networkCommands.openCommand(vorpal, p2p);
  networkCommands.connectCommand(vorpal, p2p);
  networkCommands.peersCommand(vorpal, p2p);
  networkCommands.statusCommand(vorpal, blockchain, p2p);
  networkCommands.closeServerCommand(vorpal, p2p);
  networkCommands.disconnectCommand(vorpal, p2p);
  networkCommands.disconnectAllCommand(vorpal, p2p);

  // Load Wallet Commands
  walletCommands.walletCreateCommand(vorpal, walletManager);
  walletCommands.listWalletsCommand(vorpal, walletManager, blockchain);
  walletCommands.balanceCommand(vorpal, walletManager, blockchain);

  // Load Transaction Commands
  transactionCommands.sendCommand(vorpal, walletManager, blockchain, p2p);
  transactionCommands.mempoolCommand(vorpal, blockchain);

  // Load Mining Commands
  miningCommands.mineCommand(vorpal, walletManager, blockchain, p2p);
  miningCommands.mineMempoolCommand(vorpal, walletManager, blockchain, p2p);
  miningCommands.difficultyCommand(vorpal, blockchain);

  // Load Blockchain Commands
  blockchainCommands.blockchainCommand(vorpal, blockchain);
  blockchainCommands.blockCommand(vorpal, blockchain);
  blockchainCommands.latestCommand(vorpal, blockchain);
  blockchainCommands.validateCommand(vorpal, blockchain);

  // Load Utility Commands
  utilityCommands.exportCommand(vorpal, blockchain);
  utilityCommands.importCommand(vorpal, blockchain);
  utilityCommands.clearCommand(vorpal);

  vorpal.delimiter("BLOCKCHAIN => ").show();
}

module.exports = cli;
