const Vorpal = require("vorpal");
const { P2P } = require("../p2p/P2P.js");
const { BlockChain } = require("../blockchain/BlockChain.js");

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
    .delimiter("blockchain → ")
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
      this.log(blockchain.get());
      callback();
    });
}

// Peers command: Lấy danh sách các peer đã kết nối
function peersCommand(vorpal) {
  vorpal
    .command("peers", "Get the list of connected peers.")
    .alias("p")
    .action(function (args, callback) {
      if (p2p.peers.length > 0) {
        p2p.peers.forEach((peer) => {
          this.log(`${peer._host}:${peer._port}`);
        });
      } else {
        this.log("No peers connected.");
      }
      callback();
    });
}

// Mine command: Đào block mới
function mineCommand(vorpal) {
  vorpal
    .command("mine <data>", "Mine a new block. Eg: mine hello!")
    .alias("m")
    .action(function (args, callback) {
      if (args.data) {
        blockchain.addBlock(args.data);
        p2p.broadcastNewBlock(blockchain.getLatestBlock());
        this.log("Block mined successfully!");
      } else {
        this.log("Please provide some data to mine a new block.");
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
      if (args.port) {
        const port = parseInt(args.port);
        if (Number.isInteger(port) && port > 0) {
          p2p.startServer(port);
          this.log(`Listening on port ${port}...`);
        } else {
          this.log("Invalid port! Please provide a valid number.");
        }
      } else {
        this.log("Please provide a port number.");
      }
      callback();
    });
}
