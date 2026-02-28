const WebSocket = require("ws");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("PEER_MGR");

class PeerManager {
  constructor() {
    this.peers = [];
  }

  addPeer(socket) {
    this.peers.push(socket);
  }

  removePeer(socket) {
    const index = this.peers.indexOf(socket);
    if (index > -1) {
      this.peers.splice(index, 1);
      logger.info(`Peer disconnected. Active peers: ${this.peers.length}`);
    }
  }

  disconnectPeer(index) {
    try {
      if (index >= 0 && index < this.peers.length) {
        const peer = this.peers[index];
        logger.info(`Disconnecting peer: ${peer._peerAddress}`);
        peer.close();
      } else {
        throw new Error(`Invalid peer index: ${index + 1}`);
      }
    } catch (err) {
      logger.error(`Error disconnecting peer: ${err.message}`);
      throw err;
    }
  }

  disconnectAll() {
    try {
      logger.info(`Disconnecting all ${this.peers.length} peer(s)...`);
      this.peers.forEach((peer) => {
        try {
          peer.close();
        } catch (err) {
          logger.error(`Error closing peer: ${err.message}`);
        }
      });
      this.peers = [];
      logger.success("All peers disconnected.");
    } catch (err) {
      logger.error(`Error disconnecting peers: ${err.message}`);
    }
  }

  getPeers() {
    return this.peers.map((peer) => ({
      address: peer._peerAddress || "unknown",
      state: peer.readyState === WebSocket.OPEN ? "connected" : "disconnected",
    }));
  }

  isAlreadyConnected(address) {
    return this.peers.some(
      (peer) =>
        peer._peerAddress === address || peer._peerAddress?.includes(address),
    );
  }

  sendMessage(socket, message) {
    if (socket.readyState === WebSocket.OPEN) {
      socket.send(message);
    }
  }

  broadcast(message) {
    this.peers.forEach((peer) => this.sendMessage(peer, message));
  }

  broadcastExcept(message, exceptSocket) {
    let sent = 0;
    this.peers.forEach((peer) => {
      if (peer !== exceptSocket && peer.readyState === WebSocket.OPEN) {
        peer.send(message);
        sent++;
      }
    });
    return sent;
  }
}

module.exports = PeerManager;
