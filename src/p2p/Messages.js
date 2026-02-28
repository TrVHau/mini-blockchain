const MESSAGE_TYPE = require("./message-type.js");

class Messages {
  static createMessage(type, data = null) {
    const message = { type };
    if (data !== null) {
      message.data = data;
    }
    return JSON.stringify(message);
  }

  // Block messages
  static newBlock(block) {
    return this.createMessage(MESSAGE_TYPE.NEW_BLOCK, { block });
  }

  static requestChain() {
    return this.createMessage(MESSAGE_TYPE.REQUEST_CHAIN);
  }

  static receiveChain(chain) {
    return this.createMessage(MESSAGE_TYPE.RECEIVE_CHAIN, { chain });
  }

  static requestLatest() {
    return this.createMessage(MESSAGE_TYPE.REQUEST_LATEST);
  }

  // Partial sync - request blocks từ index cụ thể
  static requestBlocksFrom(fromIndex) {
    return this.createMessage(MESSAGE_TYPE.REQUEST_BLOCKS_FROM, { fromIndex });
  }

  static receiveBlocks(blocks, fromIndex, totalHeight) {
    return this.createMessage(MESSAGE_TYPE.RECEIVE_BLOCKS, {
      blocks,
      fromIndex,
      totalHeight,
    });
  }

  // Transaction messages
  static transaction(transaction) {
    return this.createMessage(MESSAGE_TYPE.TRANSACTION, { transaction });
  }

  // Handshake messages
  static handshake(nodeInfo) {
    return this.createMessage(MESSAGE_TYPE.HANDSHAKE, nodeInfo);
  }

  static handshakeAck(nodeInfo) {
    return this.createMessage(MESSAGE_TYPE.HANDSHAKE_ACK, nodeInfo);
  }

  static parse(message) {
    try {
      return JSON.parse(message);
    } catch (err) {
      throw new Error(`Invalid message format: ${err.message}`);
    }
  }
}

module.exports = Messages;
