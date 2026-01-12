//test p2p

const { BlockChain } = require("./BlockChain");
const { Transaction } = require("./Transaction");
const { P2P } = require("./P2P");

// taoj 2 node
nodeA = new BlockChain(4);
nodeB = new BlockChain(4);
// tạo 2 peer
const P2PA = new P2P(nodeA);
const P2PB = new P2P(nodeB);

// kết nối 2 peer
P2PA.addPeer(P2PB);
P2PB.addPeer(P2PA);

// thêm vào mempool
P2PA.addTransactionToMempool(new Transaction("Alice", "Bob", 50));

// Node A mine và broadcast khối mới tới Node B
nodeA.mineMempool();
P2PA.broadcastNewBlock(nodeA.getLatesBlock());

// keiemr tra node B đã nhận được khối mới chưa
console.log("Node B Blockchain:", nodeB.get());

// đồng bộ blockchain giữa 2 node
P2PA.syncBlockchain();
console.log("After synchronization, Node B Blockchain:", nodeB.get());
