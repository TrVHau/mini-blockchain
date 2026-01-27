const MESSAGE_TYPE = require("./message-type.js");
const { Transaction } = require("../blockchain/Transaction.js");
const { CoinbaseTransaction } = require("../blockchain/CoinbaseTransaction.js");

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
          console.error("Invalid NEW_BLOCK message: missing block data");
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
          console.error("Invalid RECEIVE_CHAIN message: missing chain data");
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
      console.log(`Unknown message type: ${data.type}`);
    }
  }

  handleTransaction(data, socket) {
    try {
      if (!data.data || !data.data.transaction) {
        console.error("Invalid TRANSACTION message: missing transaction data");
        return;
      }

      const txData = data.data.transaction;

      // Validate required fields
      if (!txData.from || !txData.to || typeof txData.amount !== "number") {
        console.error("Invalid transaction: missing required fields");
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
        transaction.senderPublicKey = txData.senderPublicKey; // Quan trọng: copy PEM key
      }

      this.blockchain.addTransaction(transaction);
      console.log("Transaction added to mempool from peer");

      //  RELAY: Forward transaction to other peers (except sender)
      this.relayTransaction(transaction, socket);
    } catch (err) {
      console.error("Invalid transaction received from peer:", err.message);
    }
  }
}

module.exports = MessageHandler;
