const Messages = require("./Messages.js");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("BLOCK_HANDLER");

class BlockHandler {
  constructor(blockchain, relayBlock, requestBlockchain) {
    this.blockchain = blockchain;
    this.relayBlock = relayBlock;
    this.requestBlockchain = requestBlockchain;
  }

  handleNewBlock(block, fromSocket = null) {
    try {
      if (!block || typeof block.index === "undefined") {
        logger.error("Invalid block received");
        return;
      }

      const latestBlock = this.blockchain.getLatestBlock();

      // Nếu block nhận được là block tiếp theo
      if (block.index === latestBlock.index + 1) {
        if (this.blockchain.receiveBlock(block)) {
          logger.info(`New block #${block.index} synchronized from network`);

          //  RELAY: Forward block to other peers (except sender)
          this.relayBlock(block, fromSocket);
        } else {
          logger.warn("Block rejected, requesting full chain...");
          this.requestBlockchain();
        }
      }
      // Nếu blockchain của peer khác dài hơn nhiều
      else if (block.index > latestBlock.index + 1) {
        logger.info(
          `Blockchain behind (local: ${latestBlock.index}, received: ${block.index}). Requesting full chain...`,
        );
        this.requestBlockchain();
      }
      // Block cũ hoặc đã có
      else {
        logger.debug(`Ignoring old block #${block.index}`);
      }
    } catch (err) {
      logger.error("Error handling new block:", err.message);
    }
  }

  handleChainRequest(socket, sendMessage) {
    try {
      const message = Messages.receiveChain(this.blockchain.get());
      sendMessage(socket, message);
      logger.info("Sent blockchain to requesting peer");
    } catch (err) {
      logger.error("Error handling chain request:", err.message);
    }
  }

  handleReceiveChain(chain) {
    try {
      if (!chain || !Array.isArray(chain)) {
        logger.error("Invalid chain received");
        return;
      }
      if (this.blockchain.receiveChain(chain)) {
        logger.success("Blockchain synchronized successfully");
      }
    } catch (err) {
      logger.error("Error receiving chain:", err.message);
    }
  }

  handleLatestBlockRequest(socket, sendMessage) {
    const message = Messages.newBlock(this.blockchain.getLatestBlock());
    sendMessage(socket, message);
  }
}

module.exports = BlockHandler;
