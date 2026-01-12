class P2P {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.peers = [];
  }
  addPeer(peer) {
    this.peers.push(peer);
  }
  broadcastNewBlock(block) {
    this.peers.forEach((peer) => {
      peer.receiveNewBlock(block);
    });
  }
  receiveNewBlock(block) {
    // check valid blockchain
    if (this.blockchain.isChainValid()) {
      this.blockchain.addBlock(block.data);
      console.log("New block added from peer:", block);
    } else {
      console.log("Received invalid block from peer:", block);
    }
  }
  // gửi giao dịch vào mempool
  addTransactionToMempool(transaction) {
    this.blockchain.addToMempool(transaction);
    console.log("Transaction added to mempool:", transaction);
  }
  // đồng bộ blockchain
  syncBlockchain() {
    console.log("Synchronizing blockchain with peers...");
    this.peers.forEach((peer) => {
      peer.blockchain.receiveChain(this.blockchain.get());
    });
  }
}

module.exports = { P2P };
