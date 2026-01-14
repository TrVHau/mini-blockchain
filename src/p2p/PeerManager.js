const WebSocket = require("ws");

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
      console.log("Peer disconnected. Active peers:", this.peers.length);
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

  disconnectAll() {
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

  getPeers() {
    return this.peers.map((peer) => ({
      address: peer._peerAddress || "unknown",
      state: peer.readyState === WebSocket.OPEN ? "connected" : "disconnected",
    }));
  }

  isAlreadyConnected(address) {
    return this.peers.some(
      (peer) =>
        peer._peerAddress === address || peer._peerAddress?.includes(address)
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
