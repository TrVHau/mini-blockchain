// Network Commands: open, connect, peers, status, close-server, disconnect
const Validator = require("../../util/Validator.js");

function openCommand(vorpal, p2p) {
  vorpal
    .command(
      "open <port>",
      "Open port to accept incoming connections. Eg: open 2727"
    )
    .alias("o")
    .action(function (args, callback) {
      try {
        if (!args.port) {
          this.log("Please provide a port number.");
        } else {
          const port = Validator.validatePort(args.port);
          p2p.startServer(port);
        }
      } catch (err) {
        this.log(`Error: ${err.message}`);
      }
      callback();
    });
}

function connectCommand(vorpal, p2p) {
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

function peersCommand(vorpal, p2p) {
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

function statusCommand(vorpal, blockchain, p2p) {
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

function closeServerCommand(vorpal, p2p) {
  vorpal
    .command("close-server", "Close the P2P server.")
    .alias("cs")
    .action(function (args, callback) {
      try {
        p2p.closeServer();
      } catch (err) {
        this.log(`Error closing server: ${err.message}`);
      }
      callback();
    });
}

function disconnectCommand(vorpal, p2p) {
  vorpal
    .command(
      "disconnect <index>",
      "Disconnect a specific peer by index. Use 'peers' to see the list."
    )
    .alias("dc")
    .action(function (args, callback) {
      try {
        const index = parseInt(args.index) - 1; // -1 vì hiển thị từ 1 nhưng mảng từ 0
        if (isNaN(index) || index < 0) {
          this.log("Invalid index! Use 'peers' to see the list.");
        } else {
          p2p.disconnectPeer(index);
        }
      } catch (err) {
        this.log(`Error disconnecting peer: ${err.message}`);
      }
      callback();
    });
}

function disconnectAllCommand(vorpal, p2p) {
  vorpal
    .command("disconnect-all", "Disconnect all connected peers.")
    .alias("dca")
    .action(function (args, callback) {
      try {
        p2p.disconnectAllPeers();
      } catch (err) {
        this.log(`Error disconnecting all peers: ${err.message}`);
      }
      callback();
    });
}

module.exports = {
  openCommand,
  connectCommand,
  peersCommand,
  statusCommand,
  closeServerCommand,
  disconnectCommand,
  disconnectAllCommand,
};
