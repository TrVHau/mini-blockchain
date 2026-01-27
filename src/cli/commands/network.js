// Network Commands
const { UI, COLORS, ICONS } = require("../../util/UI.js");

function openCommand(vorpal, p2p) {
  vorpal
    .command("open <port>", "Open P2P server on port")
    .alias("o")
    .action(function (args, callback) {
      try {
        const port = parseInt(args.port);
        if (isNaN(port) || port < 1 || port > 65535) {
          this.log(UI.error("Invalid port number (1-65535)"));
        } else {
          p2p.startServer(port);
        }
      } catch (err) {
        this.log(UI.error(err.message));
      }
      callback();
    });
}

function connectCommand(vorpal, p2p) {
  vorpal
    .command("connect <host> <port>", "Connect to peer")
    .alias("c")
    .action(function (args, callback) {
      if (args.host && args.port) {
        this.log(UI.info(`Connecting to ${args.host}:${args.port}...`));
        p2p.connectToPeer(args.host, parseInt(args.port));
      }
      callback();
    });
}

function peersCommand(vorpal, p2p) {
  vorpal
    .command("peers", "List connected peers")
    .alias("p")
    .action(function (args, callback) {
      const peers = p2p.getPeers();
      if (peers.length > 0) {
        const content = peers
          .map((peer, i) => {
            const color =
              peer.state === "connected" ? COLORS.green : COLORS.red;
            return `  ${i + 1}. ${peer.address} ${color}[${peer.state}]${COLORS.reset}`;
          })
          .join("\n");
        this.log("\n" + UI.box(content, `Peers (${peers.length})`));
      } else {
        this.log(UI.warning("No peers connected"));
      }
      callback();
    });
}

function statusCommand(vorpal, blockchain, p2p) {
  vorpal
    .command("status", "Show node status")
    .alias("s")
    .action(function (args, callback) {
      const chain = blockchain.get();
      const serverPort = p2p.server ? p2p.server.address()?.port : null;
      const syncStatus = p2p.getSyncStatus();

      const content = [
        UI.keyValue(
          "Server",
          serverPort
            ? `${COLORS.green}:${serverPort}${COLORS.reset}`
            : `${COLORS.red}Offline${COLORS.reset}`,
        ),
        UI.keyValue(
          "Peers",
          `${COLORS.yellow}${p2p.peers.length}${COLORS.reset}`,
        ),
        UI.keyValue(
          "Syncing",
          syncStatus.isSyncing
            ? `${COLORS.yellow}Yes${COLORS.reset}`
            : `${COLORS.green}No${COLORS.reset}`,
        ),
        UI.divider(),
        UI.keyValue("Blocks", `${COLORS.cyan}${chain.length}${COLORS.reset}`),
        UI.keyValue(
          "Difficulty",
          `${COLORS.magenta}${blockchain.difficulty}${COLORS.reset}`,
        ),
        UI.keyValue(
          "Mempool",
          `${COLORS.yellow}${blockchain.mempool.length}${COLORS.reset} tx`,
        ),
        UI.keyValue(
          "Valid",
          blockchain.isChainValid()
            ? `${COLORS.green}Yes${COLORS.reset}`
            : `${COLORS.red}No${COLORS.reset}`,
        ),
      ].join("\n");

      this.log("\n" + UI.box(content, "Status"));
      callback();
    });
}

function syncCommand(vorpal, p2p) {
  vorpal
    .command("sync", "Sync blockchain with peers")
    .action(function (args, callback) {
      const status = p2p.getSyncStatus();
      if (status.isSyncing) {
        this.log(UI.warning("Already syncing..."));
      } else if (p2p.triggerSync()) {
        this.log(UI.success("Sync request sent"));
      }
      callback();
    });
}

module.exports = {
  openCommand,
  connectCommand,
  peersCommand,
  statusCommand,
  syncCommand,
};
