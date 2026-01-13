const MESSAGE_TYPE = require("./message-type.js");

class Messages {
  static createMessage(type, data = null) {
    const message = { type };
    if (data !== null) {
      message.data = data;
    }
    return JSON.stringify(message);
  }

  static newBlock(block) {
    return this.createMessage(MESSAGE_TYPE.NEW_BLOCK, { block });
  }

  static transaction(transaction) {
    return this.createMessage(MESSAGE_TYPE.TRANSACTION, { transaction });
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

  static requestPeers() {
    return this.createMessage(MESSAGE_TYPE.REQUEST_PEERS);
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
