const WebSocket = require("ws");
const Messages = require("./Messages.js");
const PeerManager = require("./PeerManager.js");
const BlockHandler = require("./BlockHandler.js");
const MessageHandler = require("./MessageHandler.js");
const SyncManager = require("./SyncManager.js");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");
const Validator = require("../util/Validator.js");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("P2P");

class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.server = null;
    this.serverPort = null;

    // Initialize managers
    this.peerManager = new PeerManager();
    this.syncManager = new SyncManager(blockchain);
    this.blockHandler = new BlockHandler(
      blockchain,
      this.relayBlock.bind(this),
      this.requestBlockchain.bind(this),
    );
    this.messageHandler = new MessageHandler(
      blockchain,
      this.relayBlock.bind(this),
      this.relayTransaction.bind(this),
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
        logger.error("Error processing message:", err.message);
      }
    });

    socket.on("error", (err) => {
      logger.error("Socket error:", err.message);
    });

    socket.on("close", () => {
      this.peerManager.removePeer(socket);
    });
  }

  handleMessage(socket, data) {
    // Handle sync-related messages
    if (this._handleSyncMessage(socket, data)) {
      return;
    }

    this.messageHandler.handle(socket, data, {
      handleNewBlock: this.blockHandler.handleNewBlock.bind(this.blockHandler),
      handleChainRequest: (sock) =>
        this.blockHandler.handleChainRequest(
          sock,
          this.peerManager.sendMessage.bind(this.peerManager),
        ),
      handleReceiveChain: (chain) => {
        // Delegate to SyncManager
        this.syncManager.handleReceiveChain(chain);
      },
      handleLatestBlockRequest: (sock) =>
        this.blockHandler.handleLatestBlockRequest(
          sock,
          this.peerManager.sendMessage.bind(this.peerManager),
        ),
    });
  }

  /**
   * Handle sync-specific messages
   * @returns {boolean} true if message was handled
   */
  _handleSyncMessage(socket, data) {
    const MESSAGE_TYPE = require("./message-type.js");

    switch (data.type) {
      case MESSAGE_TYPE.HANDSHAKE:
        this._handleHandshake(socket, data.data);
        return true;

      case MESSAGE_TYPE.HANDSHAKE_ACK:
        this._handleHandshakeAck(socket, data.data);
        return true;

      case MESSAGE_TYPE.REQUEST_BLOCKS_FROM:
        this._handleRequestBlocksFrom(socket, data.data);
        return true;

      case MESSAGE_TYPE.RECEIVE_BLOCKS:
        this._handleReceiveBlocks(socket, data.data);
        return true;

      default:
        return false;
    }
  }

  /**
   * Xử lý handshake từ node mới kết nối
   */
  _handleHandshake(socket, peerInfo) {
    logger.info(
      `Handshake received from peer (height: ${peerInfo.chainHeight})`,
    );

    // Gửi lại thông tin node của mình
    const myInfo = this.syncManager.getNodeInfo();
    this.peerManager.sendMessage(socket, Messages.handshakeAck(myInfo));

    // Nếu peer có chain dài hơn, bắt đầu sync
    if (peerInfo.chainHeight > this.blockchain.getLatestBlock().index) {
      this.syncManager.startSync(
        socket,
        this.peerManager.sendMessage.bind(this.peerManager),
        peerInfo.chainHeight,
      );
    }
    // Nếu mình có chain dài hơn, peer sẽ tự request khi nhận handshake_ack
  }

  /**
   * Xử lý handshake acknowledgement
   */
  _handleHandshakeAck(socket, peerInfo) {
    logger.info(
      `Handshake ACK received (peer height: ${peerInfo.chainHeight})`,
    );

    // Nếu peer có chain dài hơn, sync
    if (peerInfo.chainHeight > this.blockchain.getLatestBlock().index) {
      this.syncManager.startSync(
        socket,
        this.peerManager.sendMessage.bind(this.peerManager),
        peerInfo.chainHeight,
      );
    }
  }

  /**
   * Xử lý request blocks từ index cụ thể (partial sync)
   */
  _handleRequestBlocksFrom(socket, data) {
    const { fromIndex } = data;
    logger.info(`Peer requesting blocks from index ${fromIndex}`);

    const result = this.syncManager.getBlocksFrom(fromIndex);
    const message = Messages.receiveBlocks(
      result.blocks,
      result.fromIndex,
      result.totalHeight,
    );
    this.peerManager.sendMessage(socket, message);

    logger.info(`Sent ${result.blocks.length} blocks to peer`);
  }

  /**
   * Xử lý nhận partial blocks
   */
  _handleReceiveBlocks(socket, data) {
    const { blocks, fromIndex, totalHeight } = data;
    this.syncManager.handleReceiveBlocks(
      blocks,
      fromIndex,
      totalHeight,
      socket,
    );
  }

  connectToPeer(host, port) {
    const address = `ws://${host}:${port}`;

    // Kiểm tra tự kết nối
    if (
      this.serverPort &&
      port === this.serverPort &&
      Validator.isLocalhost(host)
    ) {
      logger.error(
        `Cannot connect to yourself! Server is running on port ${this.serverPort}`,
      );
      return;
    }

    // Kiểm tra đã kết nối chưa
    if (this.peerManager.isAlreadyConnected(address)) {
      logger.error(`Already connected to ${address}`);
      return;
    }

    logger.info(`Connecting to ${address}...`);

    try {
      const socket = new WebSocket(address, {
        handshakeTimeout: BLOCKCHAIN_CONSTANTS.WEBSOCKET_HANDSHAKE_TIMEOUT,
      });

      socket.on("open", () => {
        socket._peerAddress = address;
        this.peerManager.addPeer(socket);

        logger.success(`Connected to peer: ${address}`);

        // Gửi handshake thay vì request chain trực tiếp
        const nodeInfo = this.syncManager.getNodeInfo();
        socket.send(Messages.handshake(nodeInfo));
      });

      socket.on("error", (err) => {
        logger.error(`Failed to connect to ${address}: ${err.message}`);
      });

      socket.on("close", (code) => {
        logger.info(`Connection to ${address} closed (code: ${code})`);
      });

      this.setUpPeer(socket);
    } catch (err) {
      logger.error(`Error creating WebSocket: ${err.message}`);
    }
  }

  startServer(port) {
    try {
      if (this.server) {
        logger.warn("Server already running. Close it first.");
        return;
      }

      this.server = new WebSocket.Server({ port });
      this.serverPort = port;

      this.server.on("connection", (socket, req) => {
        socket._peerAddress =
          req.socket.remoteAddress + ":" + req.socket.remotePort;
        this.peerManager.addPeer(socket);

        logger.info(`New peer connected from ${socket._peerAddress}`);
        this.setUpPeer(socket);

        // Server cũng gửi handshake cho client mới
        const nodeInfo = this.syncManager.getNodeInfo();
        socket.send(Messages.handshake(nodeInfo));
      });

      this.server.on("error", (err) => {
        if (err.code === "EADDRINUSE") {
          logger.error(`Port ${port} is already in use!`);
        } else {
          logger.error(`Server error: ${err.message}`);
        }
        this.server = null;
        this.serverPort = null;
      });

      logger.success(`P2P server running on port ${port}`);
    } catch (err) {
      logger.error(`Failed to start server: ${err.message}`);
      this.server = null;
      this.serverPort = null;
    }
  }

  closeServer() {
    try {
      if (this.server) {
        logger.info(`Closing P2P server on port ${this.serverPort}...`);
        this.server.close(() => {
          logger.success("P2P server closed successfully.");
        });
        this.server = null;
        this.serverPort = null;
      } else {
        logger.info("No server is running.");
      }
    } catch (err) {
      logger.error(`Error closing server: ${err.message}`);
    }
  }

  // Broadcasting methods
  broadcastNewBlock(block) {
    try {
      if (this.peers.length === 0) {
        logger.debug("No peers connected to broadcast to.");
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
      logger.info(`Block broadcast to ${sent} peer(s)`);
    } catch (err) {
      logger.error("Error broadcasting block:", err.message);
    }
  }

  relayBlock(block, fromSocket) {
    try {
      if (this.peers.length === 0) return;

      const message = Messages.newBlock(block);
      const relayed = this.peerManager.broadcastExcept(message, fromSocket);

      if (relayed > 0) {
        logger.debug(`Block #${block.index} relayed to ${relayed} peer(s)`);
      }
    } catch (err) {
      logger.error("Error relaying block:", err.message);
    }
  }

  broadcastTransaction(transaction) {
    try {
      if (this.peers.length === 0) {
        logger.debug("No peers connected to broadcast to.");
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
      logger.info(`Transaction broadcast to ${sent} peer(s)`);
    } catch (err) {
      logger.error("Error broadcasting transaction:", err.message);
    }
  }

  relayTransaction(transaction, fromSocket) {
    try {
      if (this.peers.length === 0) return;

      const message = Messages.transaction(transaction);
      const relayed = this.peerManager.broadcastExcept(message, fromSocket);

      if (relayed > 0) {
        logger.debug(`Transaction relayed to ${relayed} peer(s)`);
      }
    } catch (err) {
      logger.error("Error relaying transaction:", err.message);
    }
  }

  // Helper methods
  requestBlockchain() {
    this.peerManager.broadcast(Messages.requestChain());
  }

  syncBlockchain() {
    try {
      logger.info("Requesting blockchain from peers...");
      this.requestBlockchain();
    } catch (err) {
      logger.error("Error syncing blockchain:", err.message);
    }
  }

  getPeers() {
    return this.peerManager.getPeers();
  }

  getSyncStatus() {
    return this.syncManager.getStatus();
  }

  // Manual sync trigger (for CLI command)
  triggerSync() {
    if (this.peers.length === 0) {
      logger.warn("No peers connected to sync with.");
      return false;
    }

    // Gửi handshake cho tất cả peers để trigger sync
    const nodeInfo = this.syncManager.getNodeInfo();
    this.peerManager.broadcast(Messages.handshake(nodeInfo));
    logger.info("Sync request sent to all peers");
    return true;
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
