const WebSocket = require("ws");
const Messages = require("./Messages.js");
const PeerManager = require("./PeerManager.js");
const BlockHandler = require("./BlockHandler.js");
const MessageHandler = require("./MessageHandler.js");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");

class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.server = null;
    this.serverPort = null;

    // Initialize managers
    this.peerManager = new PeerManager();
    this.blockHandler = new BlockHandler(
      blockchain,
      this.relayBlock.bind(this),
      this.requestBlockchain.bind(this)
    );
    this.messageHandler = new MessageHandler(
      blockchain,
      this.relayBlock.bind(this),
      this.relayTransaction.bind(this)
    );
  }

  // Getter for backward compatibility
  get peers() {
    return this.peerManager.peers;
  }

  setUpPeer(socket) {
    socket.on("message", (message) => {
      try {
        const data = Messages.parse(message);
        this.handleMessage(socket, data);
      } catch (err) {
        console.error("Error processing message:", err.message);
      }
    });

    socket.on("error", (err) => {
      console.error("Socket error:", err.message);
    });

    socket.on("close", () => {
      this.peerManager.removePeer(socket);
    });
  }

  handleMessage(socket, data) {
    this.messageHandler.handle(socket, data, {
      handleNewBlock: this.blockHandler.handleNewBlock.bind(this.blockHandler),
      handleChainRequest: (sock) =>
        this.blockHandler.handleChainRequest(
          sock,
          this.peerManager.sendMessage.bind(this.peerManager)
        ),
      handleReceiveChain: this.blockHandler.handleReceiveChain.bind(
        this.blockHandler
      ),
      handleLatestBlockRequest: (sock) =>
        this.blockHandler.handleLatestBlockRequest(
          sock,
          this.peerManager.sendMessage.bind(this.peerManager)
        ),
    });
  }

  connectToPeer(host, port) {
    const address = `ws://${host}:${port}`;

    // Kiểm tra tự kết nối
    if (
      this.serverPort &&
      port === this.serverPort &&
      (host === "localhost" || host === "127.0.0.1" || host === "0.0.0.0")
    ) {
      console.error(
        `Cannot connect to yourself! Server is running on port ${this.serverPort}`
      );
      return;
    }

    // Kiểm tra đã kết nối chưa
    if (this.peerManager.isAlreadyConnected(address)) {
      console.error(`Already connected to ${address}`);
      return;
    }

    console.log(`Attempting to connect to ${address}...`);

    try {
      const socket = new WebSocket(address, { 
        handshakeTimeout: BLOCKCHAIN_CONSTANTS.WEBSOCKET_HANDSHAKE_TIMEOUT 
      });

      socket.on("open", () => {
        socket._peerAddress = address;
        this.peerManager.addPeer(socket);
        console.log(`Connected to peer: ${address}`);
        this.syncBlockchain();
      });

      socket.on("error", (err) => {
        console.error(`Failed to connect to ${address}:`, err.message);
      });

      socket.on("close", (code) => {
        console.log(`Connection to ${address} closed (code: ${code})`);
      });

      this.setUpPeer(socket);
    } catch (err) {
      console.error(`Error creating WebSocket:`, err.message);
    }
  }

  startServer(port) {
    try {
      if (this.server) {
        console.log("Server already running. Close it first.");
        return;
      }

      this.server = new WebSocket.Server({ port });
      this.serverPort = port;

      this.server.on("connection", (socket, req) => {
        socket._peerAddress =
          req.socket.remoteAddress + ":" + req.socket.remotePort;
        this.peerManager.addPeer(socket);
        console.log(`New peer connected. Total peers: ${this.peers.length}`);
        this.setUpPeer(socket);
      });

      this.server.on("error", (err) => {
        if (err.code === "EADDRINUSE") {
          console.error(`Port ${port} is already in use!`);
        } else {
          console.error(`Server error:`, err.message);
        }
        this.server = null;
        this.serverPort = null;
      });

      console.log(`P2P server running on port ${port}`);
    } catch (err) {
      console.error(`Failed to start server:`, err.message);
      this.server = null;
      this.serverPort = null;
    }
  }

  closeServer() {
    try {
      if (this.server) {
        console.log(`Closing P2P server on port ${this.serverPort}...`);
        this.server.close(() => {
          console.log("P2P server closed successfully.");
        });
        this.server = null;
        this.serverPort = null;
      } else {
        console.log("No server is running.");
      }
    } catch (err) {
      console.error(`Error closing server: ${err.message}`);
    }
  }

  // Broadcasting methods
  broadcastNewBlock(block) {
    try {
      if (this.peers.length === 0) {
        console.log("No peers connected to broadcast to.");
        return;
      }

      const message = Messages.newBlock(block);
      let sent = 0;
      this.peers.forEach((peer) => {
        if (peer.readyState === WebSocket.OPEN) {
          peer.send(message);
          sent++;
        }
      });
      console.log(`Block broadcast to ${sent} peer(s)`);
    } catch (err) {
      console.error("Error broadcasting block:", err.message);
    }
  }

  relayBlock(block, fromSocket) {
    try {
      if (this.peers.length === 0) return;

      const message = Messages.newBlock(block);
      const relayed = this.peerManager.broadcastExcept(message, fromSocket);

      if (relayed > 0) {
        console.log(`  Block #${block.index} relayed to ${relayed} peer(s)`);
      }
    } catch (err) {
      console.error("Error relaying block:", err.message);
    }
  }

  broadcastTransaction(transaction) {
    try {
      if (this.peers.length === 0) {
        console.log("No peers connected to broadcast to.");
        return;
      }

      const message = Messages.transaction(transaction);
      let sent = 0;
      this.peers.forEach((peer) => {
        if (peer.readyState === WebSocket.OPEN) {
          peer.send(message);
          sent++;
        }
      });
      console.log(`Transaction broadcast to ${sent} peer(s)`);
    } catch (err) {
      console.error("Error broadcasting transaction:", err.message);
    }
  }

  relayTransaction(transaction, fromSocket) {
    try {
      if (this.peers.length === 0) return;

      const message = Messages.transaction(transaction);
      const relayed = this.peerManager.broadcastExcept(message, fromSocket);

      if (relayed > 0) {
        console.log(`  Transaction relayed to ${relayed} peer(s)`);
      }
    } catch (err) {
      console.error("Error relaying transaction:", err.message);
    }
  }

  // Helper methods
  requestBlockchain() {
    this.peerManager.broadcast(Messages.requestChain());
  }

  syncBlockchain() {
    try {
      console.log("Requesting blockchain from peers...");
      this.requestBlockchain();
    } catch (err) {
      console.error("Error syncing blockchain:", err.message);
    }
  }

  getPeers() {
    return this.peerManager.getPeers();
  }

  disconnectPeer(index) {
    this.peerManager.disconnectPeer(index);
  }

  disconnectAllPeers() {
    this.peerManager.disconnectAll();
  }

  close() {
    this.closeServer();
    this.disconnectAllPeers();
  }
}

module.exports = { P2P };
