const { createWallet } = require("./Wallet.js");
const fs = require("fs");
const path = require("path");

class WalletManager {
  constructor() {
    this.wallets = new Map();
    this.walletFileDir = path.join(__dirname, "../../data");
    this.ensureWalletDir();
    this.loadAllWallets();
  }

  // Tạo thư mục lưu wallet nếu chưa tồn tại
  ensureWalletDir() {
    if (!fs.existsSync(this.walletFileDir)) {
      fs.mkdirSync(this.walletFileDir, { recursive: true });
    }
  }

  // Tạo wallet mới và lưu vào file
  createWallet(name) {
    if (this.wallets.has(name)) {
      throw new Error("Wallet with this name already exists");
    }
    const wallet = createWallet();
    this.wallets.set(name, wallet);

    // Lưu wallet vào file
    const filePath = path.join(this.walletFileDir, `${name}.json`);
    fs.writeFileSync(filePath, JSON.stringify(wallet));
    return wallet.publicKey;
  }

  // Load wallet từ file
  loadWallet(name) {
    const filePath = path.join(this.walletFileDir, `${name}.json`);
    if (!fs.existsSync(filePath)) {
      throw new Error("Wallet file does not exist");
    }
    const walletData = fs.readFileSync(filePath);
    const wallet = JSON.parse(walletData);
    this.wallets.set(name, wallet);
    return wallet;
  }

  // Load tất cả wallet từ file
  loadAllWallets() {
    if (!fs.existsSync(this.walletFileDir)) {
      return;
    }
    const files = fs.readdirSync(this.walletFileDir);
    files.forEach((file) => {
      if (file.endsWith(".json")) {
        const name = file.replace(".json", "");
        try {
          this.loadWallet(name);
        } catch (error) {
          console.error(`Failed to load wallet ${name}:`, error);
        }
      }
    });
  }
  // List tất cả wallet đã load
  listWallets() {
    return Array.from(this.wallets.keys());
  }
  getPublicKey(name) {
    const wallet = this.wallets.get(name);
    if (!wallet) {
      throw new Error("Wallet not found");
    }
    return wallet.publicKey;
  }
  getPrivateKey(name) {
    const wallet = this.wallets.get(name);
    if (!wallet) {
      throw new Error("Wallet not found");
    }
    return wallet.privateKey;
  }
  hasWallet(name) {
    return this.wallets.has(name);
  }
}
module.exports = { WalletManager };
