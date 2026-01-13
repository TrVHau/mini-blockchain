const WebSocket = require("ws");
const Messages = require("./Messages.js");
const MESSAGE_TYPE = require("./message-type.js");

class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.peers = [];
    this.server = null;
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
      // Remove peer from peers array
      const index = this.peers.indexOf(socket);
      if (index > -1) {
        this.peers.splice(index, 1);
        console.log("Peer disconnected. Active peers:", this.peers.length);
      }
    });
  }

  handleMessage(socket, data) {
    const handlers = {
      [MESSAGE_TYPE.NEW_BLOCK]: () => this.handleNewBlock(data.data.block),
      [MESSAGE_TYPE.TRANSACTION]: () => {
        // theem vaof mempool neu valid
        try {
          this.blockchain.addTransaction(data.data.transaction);
          console.log("Transaction added to mempool from peer");
        } catch (err) {
          console.error("Invalid transaction received from peer:", err.message);
        }
      },
      [MESSAGE_TYPE.REQUEST_CHAIN]: () => this.handleChainRequest(socket),
      [MESSAGE_TYPE.RECEIVE_CHAIN]: () =>
        this.handleReceiveChain(data.data.chain),
      [MESSAGE_TYPE.REQUEST_LATEST]: () =>
        this.handleLatestBlockRequest(socket),
    };

    const handler = handlers[data.type];
    if (handler) {
      handler();
    } else {
      console.log(`Unknown message type: ${data.type}`);
    }
  }

  connectToPeer(host, port) {
    const address = `ws://${host}:${port}`;
    console.log(`Attempting to connect to ${address}...`);

    try {
      const socket = new WebSocket(address, {
        handshakeTimeout: 5000, // 5 seconds timeout
      });

      socket.on("open", () => {
        socket._peerAddress = address;
        this.peers.push(socket);
        console.log(`Connected to peer: ${address}`);
        this.syncBlockchain();
      });

      socket.on("error", (err) => {
        console.error(`Failed to connect to ${address}:`, err.message);
        console.log(`   Possible reasons:
   - Peer is not running
   - Wrong host/port
   - Firewall blocking connection
   - Network unreachable`);
      });

      socket.on("close", (code, reason) => {
        console.log(`Connection to ${address} closed (code: ${code})`);
      });

      this.setUpPeer(socket);
    } catch (err) {
      console.error(`Error creating WebSocket:`, err.message);
    }
  }

  handleNewBlock(block) {
    try {
      if (!block || typeof block.index === "undefined") {
        console.error("Invalid block received");
        return;
      }

      const latestBlock = this.blockchain.getLatestBlock();

      // Nếu block nhận được là block tiếp theo
      if (block.index === latestBlock.index + 1) {
        if (this.blockchain.receiveBlock(block)) {
          console.log(`New block #${block.index} synchronized from network`);
        } else {
          console.log("Block rejected, requesting full chain...");
          this.requestBlockchain();
        }
      }
      // Nếu blockchain của peer khác dài hơn nhiều
      else if (block.index > latestBlock.index + 1) {
        console.log(
          `Blockchain seems to be behind (local: ${latestBlock.index}, received: ${block.index})`
        );
        console.log("   Requesting full blockchain...");
        this.requestBlockchain();
      }
      // Block cũ hoặc đã có
      else {
        console.log(`Ignoring old block #${block.index}`);
      }
    } catch (err) {
      console.error("Error handling new block:", err.message);
    }
  }

  handleChainRequest(socket) {
    try {
      const message = Messages.receiveChain(this.blockchain.get());
      this.sendMessage(socket, message);
      console.log("Sent blockchain to requesting peer");
    } catch (err) {
      console.error("Error handling chain request:", err.message);
    }
  }

  handleReceiveChain(chain) {
    try {
      if (!chain || !Array.isArray(chain)) {
        console.error("Invalid chain received");
        return;
      }
      if (this.blockchain.receiveChain(chain)) {
        console.log("Blockchain synchronized successfully");
      }
    } catch (err) {
      console.error("Error receiving chain:", err.message);
    }
  }

  handleLatestBlockRequest(socket) {
    const message = Messages.newBlock(this.blockchain.getLatestBlock());
    this.sendMessage(socket, message);
  }

  sendMessage(socket, message) {
    if (socket.readyState === WebSocket.OPEN) {
      socket.send(message);
    }
  }

  broadcast(message) {
    this.peers.forEach((peer) => this.sendMessage(peer, message));
  }

  requestBlockchain() {
    this.broadcast(Messages.requestChain());
  }

  startServer(port) {
    try {
      if (this.server) {
        console.log("Server already running. Close it first.");
        return;
      }

      this.server = new WebSocket.Server({ port });

      this.server.on("connection", (socket, req) => {
        socket._peerAddress =
          req.socket.remoteAddress + ":" + req.socket.remotePort;
        this.peers.push(socket);
        console.log(`New peer connected. Total peers: ${this.peers.length}`);
        this.setUpPeer(socket);
      });

      this.server.on("error", (err) => {
        if (err.code === "EADDRINUSE") {
          console.error(`Port ${port} is already in use!`);
          console.log(
            `   Try another port or close the application using this port.`
          );
        } else {
          console.error(`Server error:`, err.message);
        }
        this.server = null;
      });

      console.log(`P2P server running on port ${port}`);
    } catch (err) {
      console.error(`Failed to start server:`, err.message);
      this.server = null;
    }
  }

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

  discoverPeers() {
    try {
      if (this.peers.length === 0) {
        console.log("No peers connected to discover from.");
        return;
      }

      console.log("Discovering peers...");
      this.broadcast(Messages.requestPeers());
    } catch (err) {
      console.error("Error discovering peers:", err.message);
    }
  }

  syncBlockchain() {
    try {
      if (this.peers.length === 0) {
        console.log("No peers to sync with.");
        return;
      }
      console.log("Requesting blockchain from peers...");
      this.requestBlockchain();
    } catch (err) {
      console.error("Error syncing blockchain:", err.message);
    }
  }

  getPeers() {
    return this.peers.map((peer) => ({
      address: peer._peerAddress || "unknown",
      state: peer.readyState === WebSocket.OPEN ? "connected" : "disconnected",
    }));
  }

  close() {
    if (this.server) {
      this.server.close();
    }
    this.peers.forEach((peer) => peer.close());
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
}

module.exports = { P2P };
