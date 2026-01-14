const Messages = require("./Messages.js");

class BlockHandler {
  constructor(blockchain, relayBlock, requestBlockchain) {
    this.blockchain = blockchain;
    this.relayBlock = relayBlock;
    this.requestBlockchain = requestBlockchain;
  }

  handleNewBlock(block, fromSocket = null) {
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

          //  RELAY: Forward block to other peers (except sender)
          this.relayBlock(block, fromSocket);
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

  handleChainRequest(socket, sendMessage) {
    try {
      const message = Messages.receiveChain(this.blockchain.get());
      sendMessage(socket, message);
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

  handleLatestBlockRequest(socket, sendMessage) {
    const message = Messages.newBlock(this.blockchain.getLatestBlock());
    sendMessage(socket, message);
  }
}

module.exports = BlockHandler;
