const Vorpal = require("vorpal");
const { P2P } = require("../p2p/P2P.js");
const { BlockChain } = require("../blockchain/BlockChain.js");
const { WalletManager } = require("../wallet/WalletManager.js");
const { Transaction } = require("../blockchain/Transaction.js");
const { shortenAddress } = require("../util/AddressHelper.js");

const walletManager = new WalletManager(); // thêm ví quản lý ví
const blockchain = new BlockChain();
const p2p = new P2P(blockchain);

// Khởi tạo Vorpal CLI
function cli() {
  const vorpal = Vorpal();

  // Tải các command vào
  vorpal
    .use(welcome)
    .use(connectCommand)
    .use(discoverCommand)
    .use(blockchainCommand)
    .use(peersCommand)
    .use(mineCommand)
    .use(openCommand)
    .use(statusCommand)
    .use(validateCommand)
    .use(blockCommand)
    .use(latestCommand)
    .use(difficultyCommand)
    .use(mempoolCommand)
    .use(addTransactionCommand)
    .use(mineMempoolCommand)
    .use(exportCommand)
    .use(importCommand)
    .use(clearCommand)
    .use(walletCreateCommand)
    .use(listWalletsCommand)
    .use(balanceCommand)
    .use(sendCommand)
    .use(allBalancesCommand)
    .delimiter("BLOCKCHAIN => ")
    .show();
}

module.exports = cli;

// COMMANDS

// Welcome message
function welcome(vorpal) {
  vorpal.log("Welcome to Blockchain CLI!");
  vorpal.exec("help");
}

// Connect command: Kết nối đến peer
function connectCommand(vorpal) {
  vorpal
    .command(
      "connect <host> <port>",
      "Connect to a new peer. Eg: connect localhost 2727"
    )
    .alias("c")
    .action(function (args, callback) {
      if (args.host && args.port) {
        try {
          p2p.connectToPeer(args.host, args.port);
        } catch (err) {
          this.log("Error: " + err);
        }
      }
      callback();
    });
}

// Discover command: Khám phá các peer mới từ peer đã kết nối
function discoverCommand(vorpal) {
  vorpal
    .command("discover", "Discover new peers from your connected peers.")
    .alias("d")
    .action(function (args, callback) {
      try {
        p2p.discoverPeers();
      } catch (err) {
        this.log("Error: " + err);
      }
      callback();
    });
}

// Blockchain command: Xem trạng thái của blockchain hiện tại
function blockchainCommand(vorpal) {
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

// Peers command: Lấy danh sách các peer đã kết nối
function peersCommand(vorpal) {
  vorpal
    .command("peers", "Get the list of connected peers.")
    .alias("p")
    .action(function (args, callback) {
      const peers = p2p.getPeers();
      if (peers.length > 0) {
        this.log(`\nConnected Peers (${peers.length}):\n`);
        peers.forEach((peer, i) => {
          this.log(`${i + 1}. ${peer.address} [${peer.state}]`);
        });
        this.log("");
      } else {
        this.log("No peers connected.");
      }
      callback();
    });
}

// Mine command: Đào block mới
function mineCommand(vorpal) {
  vorpal
    .command("mine <wallet> [data]", "Mine a new block. Eg: mine hello!")
    .alias("m")
    .action(function (args, callback) {
      try {
        // lấy miner address
        const minerAddress = walletManager.getPublicKey(args.wallet);
        const data = args.data || "";

        // đào block mới
        this.log(`\nMining new block for wallet: ${args.wallet}...`);
        const block = blockchain.mineBlock(minerAddress, data);

        // broadcast block mới đến các peer
        p2p.broadcastNewBlock(block);
        const reward = blockchain.getMiningReward();

        this.log(`\nBlock mined successfully!`);
        this.log(`Block #${block.index}`);
        this.log(`Hash: ${block.hash}`);
        this.log(`Miner: ${shortenAddress(minerAddress)}`);
        this.log(`Reward: ${reward} coins`);
        this.log(`Transactions: ${block.transactions.length}`);
        this.log(`Total Fees: ${block.totalFees} coins\n`);
      } catch (err) {
        this.log(`Error mining block: ${err.message}`);
      }
      callback();
    });
}

// Open command: Mở cổng để chấp nhận kết nối từ peer
function openCommand(vorpal) {
  vorpal
    .command(
      "open <port>",
      "Open port to accept incoming connections. Eg: open 2727"
    )
    .alias("o")
    .action(function (args, callback) {
      try {
        if (args.port) {
          const port = parseInt(args.port);
          if (Number.isInteger(port) && port > 0 && port <= 65535) {
            p2p.startServer(port);
          } else {
            this.log("Invalid port! Must be between 1 and 65535.");
          }
        } else {
          this.log("Please provide a port number.");
        }
      } catch (err) {
        this.log(`Error opening port: ${err.message}`);
      }
      callback();
    });
}

// Status command: Xem trạng thái tổng quan
function statusCommand(vorpal) {
  vorpal
    .command("status", "Show node status and statistics.")
    .alias("s")
    .action(function (args, callback) {
      try {
        const chain = blockchain.get();
        let serverPort = "Not opened";

        if (p2p.server) {
          try {
            serverPort = p2p.server.address().port;
          } catch (err) {
            serverPort = "Error";
          }
        }

        this.log("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        this.log("         NODE STATUS");
        this.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        this.log(`Server Port     : ${serverPort}`);
        this.log(`Connected Peers : ${p2p.peers.length}`);
        this.log(`Blockchain Size : ${chain.length} blocks`);
        this.log(`Latest Block    : #${chain[chain.length - 1].index}`);
        this.log(`Mining Difficulty: ${blockchain.difficulty}`);
        this.log(`Mempool Size    : ${blockchain.mempool.length} transactions`);
        this.log(
          `Chain Valid     : ${blockchain.isChainValid() ? "Yes" : "No"}`
        );
        this.log("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
      } catch (err) {
        this.log(`Error getting status: ${err.message}`);
      }
      callback();
    });
}

// Validate command: Kiểm tra tính hợp lệ của blockchain
function validateCommand(vorpal) {
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

// Block command: Xem chi tiết một block
function blockCommand(vorpal) {
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

// Latest command: Xem block mới nhất
function latestCommand(vorpal) {
  vorpal
    .command("latest", "View the latest block in the chain.")
    .alias("l")
    .action(function (args, callback) {
      const latestBlock = blockchain.getLatestBlock();
      this.log("\n" + latestBlock.toString());
      callback();
    });
}

// Difficulty command: Thay đổi độ khó đào
function difficultyCommand(vorpal) {
  vorpal
    .command(
      "difficulty <level>",
      "Set mining difficulty (1-6). Eg: difficulty 3"
    )
    .action(function (args, callback) {
      const level = parseInt(args.level);
      if (level >= 1 && level <= 6) {
        blockchain.difficulty = level;
        this.log(` Mining difficulty set to ${level}`);
        this.log(`   Blocks must start with ${"0".repeat(level)}...`);
      } else {
        this.log(" Difficulty must be between 1 and 6");
      }
      callback();
    });
}

// Mempool command: Xem mempool
function mempoolCommand(vorpal) {
  vorpal
    .command("mempool", "View pending transactions in mempool.")
    .action(function (args, callback) {
      if (blockchain.mempool.length === 0) {
        this.log("Mempool is empty.");
      } else {
        this.log(`\nMempool (${blockchain.mempool.length} transactions):\n`);
        blockchain.mempool.forEach((tx, i) => {
          this.log(`${i + 1}. ${JSON.stringify(tx)}`);
        });
      }
      callback();
    });
}

// Add Transaction command: Thêm transaction vào mempool
function addTransactionCommand(vorpal) {
  vorpal
    .command(
      "add-tx <from> <to> <amount>",
      "Add transaction to mempool. Eg: add-tx Alice Bob 10"
    )
    .action(function (args, callback) {
      try {
        const amount = parseFloat(args.amount);

        if (isNaN(amount) || amount <= 0) {
          this.log("Invalid amount! Must be a positive number.");
          callback();
          return;
        }

        const transaction = {
          from: args.from,
          to: args.to,
          amount: amount,
          timestamp: Date.now(),
        };
        blockchain.addToMempool(transaction);
        this.log(
          `Transaction added to mempool: ${args.from} → ${args.to} (${args.amount})`
        );
      } catch (err) {
        this.log(`Error adding transaction: ${err.message}`);
      }
      callback();
    });
}

// Mine Mempool command: Đào các transaction trong mempool
function mineMempoolCommand(vorpal) {
  vorpal
    .command("mine-mempool", "Mine all pending transactions in mempool.")
    .alias("mm")
    .action(function (args, callback) {
      try {
        if (blockchain.mempool.length === 0) {
          this.log("Mempool is empty, nothing to mine.");
        } else {
          this.log(`Mining ${blockchain.mempool.length} transactions...`);
          blockchain.mineMempool();
          p2p.broadcastNewBlock(blockchain.getLatestBlock());
          this.log("Block mined and broadcasted successfully!");
        }
      } catch (err) {
        this.log(`Error mining mempool: ${err.message}`);
      }
      callback();
    });
}

// Export command: Export blockchain ra file
function exportCommand(vorpal) {
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

// Import command: Import blockchain từ file
function importCommand(vorpal) {
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

// Clear command: Xóa màn hình
function clearCommand(vorpal) {
  vorpal
    .command("clear", "Clear the terminal screen.")
    .action(function (args, callback) {
      console.clear();
      this.log("Welcome to Blockchain CLI!");
      callback();
    });
}

function walletCreateCommand(vorpal) {
  vorpal
    .command(
      "wallet-create <name>",
      "Create a new wallet. Eg: wallet-create Alice"
    )
    .alias("wc")
    .action(function (args, callback) {
      try {
        const publicKey = walletManager.createWallet(args.name);
        this.log(`\nWallet created: ${args.name}`);
        this.log(`Address: ${shortenAddress(publicKey)}\n`);
      } catch (err) {
        this.log(`Error creating wallet: ${err.message}`);
      }
      callback();
    });
}
function listWalletsCommand(vorpal) {
  vorpal
    .command("wallets", "List all wallets.")
    .alias("wl")
    .action(function (args, callback) {
      try {
        const wallets = walletManager.listWallets();
        if (wallets.length === 0) {
          this.log("No wallets found.");
        } else {
          this.log("\nWallets:\n");
          wallets.forEach((name, i) => {
            const address = walletManager.getPublicKey(name);
            const balance = blockchain.getBalance(address);
            this.log(`${i + 1}. ${name}`);
            this.log(`   Address: ${shortenAddress(address)}`);
            this.log(`   Balance: ${balance} coins`);
            this.log("-----------------------");
          });
        }
      } catch (err) {
        this.log(`Error listing wallets: ${err.message}`);
      }
      callback();
    });
}

// check balance command
function balanceCommand(vorpal) {
  vorpal
    .command("balance <name>", "Check balance of a wallet. Eg: balance Alice")
    .alias("bal")
    .action(function (args, callback) {
      try {
        const address = walletManager.getPublicKey(args.name);
        const balance = blockchain.getBalance(address);
        this.log(`\nWallet: ${args.name}`);
        this.log(`Address: ${shortenAddress(address)}`);
        this.log(`Balance: ${balance} coins\n`);
      } catch (err) {
        this.log(`Error checking balance: ${err.message}`);
      }
      callback();
    });
}

// send transaction command
function sendCommand(vorpal) {
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
        const amount = parseFloat(args.amount);
        const fee = args.fee ? parseFloat(args.fee) : 0;
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

function allBalancesCommand(vorpal) {
  vorpal
    .command("all-balances", "View all wallet balances.")
    .alias("ab")
    .action(function (args, callback) {
      try {
        const balances = blockchain.getAllBalances();
        const entries = Object.entries(balances);
        if (entries.length === 0) {
          this.log("No addresses with balance.");
        } else {
          this.log("\nAll Wallet Balances:\n");
          entries.forEach(([address, balance]) => {
            this.log(`Address: ${shortenAddress(address)}`);
            this.log(`Balance: ${balance} coins`);
            this.log("-----------------------");
          });
        }
      } catch (err) {
        this.log(`Error getting all balances: ${err.message}`);
      }
      callback();
    });
}
