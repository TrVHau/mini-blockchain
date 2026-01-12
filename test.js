// test-sign.js
const { createWallet } = require("./Wallet");
const { Transaction } = require("./Transaction");

const alice = createWallet();
const bob = createWallet();

const tx = new Transaction(alice.publicKey, bob.publicKey, 10);
tx.sign(alice.privateKey);

console.log("Valid tx?", tx.isValid(alice.publicKey)); // true

// thử giả mạo
tx.amount = 1000;
console.log("Valid after tamper?", tx.isValid(alice.publicKey)); // false
