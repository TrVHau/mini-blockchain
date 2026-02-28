const Vorpal = require("vorpal");
const { P2P } = require("../p2p/P2P.js");
const { BlockChain } = require("../blockchain/BlockChain.js");
const { WalletManager } = require("../wallet/WalletManager.js");
const { UI, COLORS } = require("../util/UI.js");
const Storage = require("../storage/Storage.js");

// Import command modules
const networkCommands = require("./commands/network.js");
const walletCommands = require("./commands/wallet.js");
const transactionCommands = require("./commands/transaction.js");
const miningCommands = require("./commands/mining.js");
const blockchainCommands = require("./commands/blockchain.js");

/**
 * Khởi tạo CLI với options
 */
function cli(options = {}) {
  const nodeId = options.nodeId || "default";

  // Khởi tạo storage và blockchain
  const storage = new Storage(nodeId);
  const blockchain = new BlockChain();

  // Load blockchain từ storage
  const savedChain = storage.loadBlockchain();
  if (savedChain && savedChain.length > 1) {
    if (blockchain.receiveChain(savedChain)) {
      console.log(`Loaded ${savedChain.length} blocks from storage`);
    }
  }

  // Khởi tạo wallet và P2P
  const walletManager = new WalletManager(nodeId);
  const p2p = new P2P(blockchain);

  // Auto-save khi blockchain thay đổi
  const wrapWithSave = (fn) =>
    function (...args) {
      const result = fn.apply(this, args);
      if (result) storage.saveBlockchain(blockchain.get());
      return result;
    };

  blockchain.mineBlock = wrapWithSave(blockchain.mineBlock.bind(blockchain));
  blockchain.receiveBlock = wrapWithSave(
    blockchain.receiveBlock.bind(blockchain),
  );
  blockchain.receiveChain = wrapWithSave(
    blockchain.receiveChain.bind(blockchain),
  );

  // Khởi tạo CLI
  const vorpal = Vorpal();

  // Banner
  console.log(UI.banner());
  console.log(`  ${COLORS.dim}Node: ${COLORS.cyan}${nodeId}${COLORS.reset}`);
  console.log(
    `  ${COLORS.dim}Chain: ${COLORS.yellow}${blockchain.get().length}${COLORS.reset} blocks\n`,
  );

  // Network Commands
  networkCommands.openCommand(vorpal, p2p);
  networkCommands.connectCommand(vorpal, p2p);
  networkCommands.peersCommand(vorpal, p2p);
  networkCommands.statusCommand(vorpal, blockchain, p2p);
  networkCommands.syncCommand(vorpal, p2p);
  networkCommands.closeCommand(vorpal, p2p);
  networkCommands.disconnectCommand(vorpal, p2p);

  // Wallet Commands
  walletCommands.walletCreateCommand(vorpal, walletManager);
  walletCommands.listWalletsCommand(vorpal, walletManager, blockchain);
  walletCommands.balanceCommand(vorpal, walletManager, blockchain);
  walletCommands.addressCommand(vorpal, walletManager);
  walletCommands.historyCommand(vorpal, walletManager, blockchain);
  walletCommands.exportCommand(vorpal, walletManager);
  walletCommands.importCommand(vorpal, walletManager);
  walletCommands.deleteWalletCommand(vorpal, walletManager);

  // Transaction Commands
  transactionCommands.sendCommand(vorpal, walletManager, blockchain, p2p);

  // Mining Commands
  miningCommands.mineCommand(vorpal, walletManager, blockchain, p2p);
  miningCommands.autoMineCommand(vorpal, walletManager, blockchain, p2p);
  miningCommands.stopAutoMineCommand(vorpal);

  // Blockchain Commands
  blockchainCommands.blockchainCommand(vorpal, blockchain);
  blockchainCommands.blockCommand(vorpal, blockchain);
  blockchainCommands.latestCommand(vorpal, blockchain);
  blockchainCommands.validateCommand(vorpal, blockchain);
  blockchainCommands.statsCommand(vorpal, blockchain);
  blockchainCommands.txCommand(vorpal, blockchain);
  blockchainCommands.mempoolCommand(vorpal, blockchain);
  blockchainCommands.feeCommand(vorpal, blockchain);
  blockchainCommands.resetCommand(vorpal, blockchain);

  // Auto-start server
  if (options.autoStart && options.port) {
    p2p.startServer(options.port);
  }

  // Auto-connect
  if (options.connect) {
    const [host, port] = options.connect.split(":");
    if (host && port) {
      setTimeout(() => p2p.connectToPeer(host, parseInt(port)), 1000);
    }
  }

  // Prompt
  vorpal
    .delimiter(`${COLORS.cyan}⛓ ${nodeId} ${COLORS.yellow}➜${COLORS.reset} `)
    .show();

  // Cleanup
  process.on("SIGINT", () => {
    storage.saveBlockchain(blockchain.get());
    p2p.close();
    process.exit(0);
  });
}

module.exports = cli;
