const WebSocket = require("ws");
const Messages = require("./Messages.js");
const MESSAGE_TYPE = require("./message-type.js");
const { Transaction } = require("../blockchain/Transaction.js");
const { CoinbaseTransaction } = require("../blockchain/CoinbaseTransaction.js");

class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.peers = [];
    this.server = null;
    this.serverPort = null; // Lưu port server đang chạy
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
      [MESSAGE_TYPE.NEW_BLOCK]: () => {
        if (!data.data || !data.data.block) {
          console.error("Invalid NEW_BLOCK message: missing block data");
          return;
        }
        this.handleNewBlock(data.data.block);
      },
      [MESSAGE_TYPE.TRANSACTION]: () => {
        // Thêm vào mempool nếu valid
        try {
          if (!data.data || !data.data.transaction) {
            console.error(
              "Invalid TRANSACTION message: missing transaction data"
            );
            return;
          }

          const txData = data.data.transaction;

          // Validate required fields
          if (!txData.from || !txData.to || typeof txData.amount !== "number") {
            console.error("Invalid transaction: missing required fields");
            return;
          }

          let transaction;

          // Recreate transaction instance from JSON
          if (txData.type === "COINBASE") {
            transaction = new CoinbaseTransaction(txData.to, txData.amount);
          } else {
            transaction = new Transaction(
              txData.from,
              txData.to,
              txData.amount,
              txData.fee
            );
            transaction.signature = txData.signature;
            transaction.timestamp = txData.timestamp;
          }

          // Lấy public key từ address (from)
          const senderPublicKey = txData.from; // Giả sử from chính là public key

          this.blockchain.addTransaction(transaction, senderPublicKey);
          console.log("Transaction added to mempool from peer");
        } catch (err) {
          console.error("Invalid transaction received from peer:", err.message);
        }
      },
      [MESSAGE_TYPE.REQUEST_CHAIN]: () => this.handleChainRequest(socket),
      [MESSAGE_TYPE.RECEIVE_CHAIN]: () => {
        if (!data.data || !data.data.chain) {
          console.error("Invalid RECEIVE_CHAIN message: missing chain data");
          return;
        }
        this.handleReceiveChain(data.data.chain);
      },
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

    // Kiểm tra xem có đang cố kết nối tới chính mình không
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

    // Kiểm tra xem đã kết nối tới peer này chưa
    const alreadyConnected = this.peers.some((peer) => {
      return (
        peer._peerAddress === address ||
        peer._peerAddress?.includes(`${host}:${port}`)
      );
    });

    if (alreadyConnected) {
      console.error(`Already connected to ${address}`);
      return;
    }

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
      this.serverPort = port; // Lưu port server

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
        this.serverPort = null;
      });

      console.log(`P2P server running on port ${port}`);
    } catch (err) {
      console.error(`Failed to start server:`, err.message);
      this.server = null;
      this.serverPort = null;
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

  disconnectPeer(index) {
    try {
      if (index >= 0 && index < this.peers.length) {
        const peer = this.peers[index];
        console.log(`Disconnecting peer: ${peer._peerAddress}`);
        peer.close();
        // Peer sẽ tự động bị xóa khỏi mảng qua event 'close'
      } else {
        console.log(`Invalid peer index: ${index}`);
      }
    } catch (err) {
      console.error(`Error disconnecting peer: ${err.message}`);
    }
  }

  disconnectAllPeers() {
    try {
      console.log(`Disconnecting all ${this.peers.length} peer(s)...`);
      this.peers.forEach((peer) => {
        try {
          peer.close();
        } catch (err) {
          console.error(`Error closing peer: ${err.message}`);
        }
      });
      this.peers = [];
      console.log("All peers disconnected.");
    } catch (err) {
      console.error(`Error disconnecting peers: ${err.message}`);
    }
  }

  close() {
    this.closeServer();
    this.disconnectAllPeers();
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
