const MESSAGE_TYPE = require("./message-type.js");
const { Transaction } = require("../blockchain/Transaction.js");
const { CoinbaseTransaction } = require("../blockchain/CoinbaseTransaction.js");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("MSG_HANDLER");

class MessageHandler {
  constructor(blockchain, relayBlock, relayTransaction) {
    this.blockchain = blockchain;
    this.relayBlock = relayBlock;
    this.relayTransaction = relayTransaction;
  }

  handle(socket, data, handlers) {
    const messageHandlers = {
      [MESSAGE_TYPE.NEW_BLOCK]: () => {
        if (!data.data || !data.data.block) {
          logger.error("Invalid NEW_BLOCK message: missing block data");
          return;
        }
        handlers.handleNewBlock(data.data.block, socket);
      },

      [MESSAGE_TYPE.TRANSACTION]: () => {
        this.handleTransaction(data, socket);
      },

      [MESSAGE_TYPE.REQUEST_CHAIN]: () => handlers.handleChainRequest(socket),

      [MESSAGE_TYPE.RECEIVE_CHAIN]: () => {
        if (!data.data || !data.data.chain) {
          logger.error("Invalid RECEIVE_CHAIN message: missing chain data");
          return;
        }
        handlers.handleReceiveChain(data.data.chain);
      },

      [MESSAGE_TYPE.REQUEST_LATEST]: () =>
        handlers.handleLatestBlockRequest(socket),
    };

    const handler = messageHandlers[data.type];
    if (handler) {
      handler();
    } else {
      logger.debug(`Unhandled message type: ${data.type}`);
    }
  }

  handleTransaction(data, socket) {
    try {
      if (!data.data || !data.data.transaction) {
        logger.error("Invalid TRANSACTION message: missing transaction data");
        return;
      }

      const txData = data.data.transaction;

      // Validate required fields
      if (!txData.from || !txData.to || typeof txData.amount !== "number") {
        logger.error("Invalid transaction: missing required fields");
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
          txData.fee,
        );
        transaction.signature = txData.signature;
        transaction.timestamp = txData.timestamp;
        transaction.senderPublicKey = txData.senderPublicKey;
        transaction.type = txData.type || "TRANSFER";
        // Reconstruct txid from the original transaction data
        transaction.txid = txData.txid || transaction.calculateTxid();
      }

      this.blockchain.addTransaction(transaction);
      logger.info("Transaction added to mempool from peer");

      //  RELAY: Forward transaction to other peers (except sender)
      this.relayTransaction(transaction, socket);
    } catch (err) {
      logger.debug(`Transaction from peer rejected: ${err.message}`);
    }
  }
}

module.exports = MessageHandler;
