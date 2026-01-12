// test mempool addition

const { BlockChain } = require("./BlockChain");
const blockchain = new BlockChain(4);

// add transactions to mempool
blockchain.addToMempool({ from: "Alice", to: "Bob", amount: 50 });
blockchain.addToMempool({ from: "Charlie", to: "Dave", amount: 25 });
blockchain.addToMempool({ from: "Eve", to: "Frank", amount: 75 });

console.log("Current Mempool:", blockchain.mempool);

blockchain.mineMempool();

console.log("Blockchain after mining mempool:");
console.log(blockchain.chain);
