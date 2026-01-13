const WebSocket = require("ws");

class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.peers = [];
    this.server = null;
  }

  setUpPeer(socket) {
    socket.on("message", (message) => {
      const data = JSON.parse(message);
      if (data.type === "NEW_BLOCK") {
        this.receiveNewBlock(data.block);
      } else if (data.type === "TRANSACTION") {
        this.blockchain.addToMempool(data.transaction);
      }
    });
  }

  connectToPeer(host, port) {
    const address = `ws://${host}:${port}`;
    const socket = new WebSocket(address);
    socket.on("open", () => {
      this.peers.push(socket);
      console.log(`Connected to peer: ${address}`);
      this.syncBlockchain();
    });

    this.setUpPeer(socket);
  }

  receiveNewBlock(block) {
    this.blockchain.addBlock(block.data);
  }

  startServer(port) {
    this.server = new WebSocket.Server({ port });
    this.server.on("connection", (socket) => {
      this.peers.push(socket);
      console.log("New peer connected");
      this.setUpPeer(socket);
    });
    console.log(`P2P server running on port ${port}`);
  }

  broadcastNewBlock(block) {
    const message = JSON.stringify({ type: "NEW_BLOCK", block });
    this.peers.forEach((peer) => {
      if (peer.readyState === WebSocket.OPEN) {
        peer.send(message);
      }
    });
  }

  discoverPeers() {
    console.log("Discovering peers...");
    const message = JSON.stringify({ type: "REQUEST_PEERS" });
    this.peers.forEach((peer) => {
      if (peer.readyState === WebSocket.OPEN) {
        peer.send(message);
      }
    });
  }

  syncBlockchain() {
    console.log("Syncing blockchain with peers...");
    this.peers.forEach((peer) => {
      if (peer.readyState === WebSocket.OPEN) {
        const message = JSON.stringify({
          type: "BLOCKCHAIN_SYNC",
          chain: this.blockchain.get(),
        });
        peer.send(message);
      }
    });
  }

  close() {
    if (this.server) {
      this.server.close();
    }
    this.peers.forEach((peer) => peer.close());
  }
}

module.exports = { P2P };
