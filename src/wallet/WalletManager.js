const { createWallet } = require("./Wallet.js");
const { publicKeyToAddress } = require("../util/AddressHelper.js");
const fs = require("fs");
const path = require("path");

class WalletManager {
  constructor(nodeId = "default") {
    this.wallets = new Map();
    this.nodeId = nodeId;

    // Node-specific wallet directory
    if (nodeId === "default") {
      this.walletFileDir = path.join(process.cwd(), "data");
    } else {
      this.walletFileDir = path.join(
        process.cwd(),
        "data",
        "nodes",
        nodeId,
        "wallets",
      );
    }

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
    // Tạo hash address từ public key
    wallet.address = publicKeyToAddress(wallet.publicKey);
    this.wallets.set(name, wallet);

    // Lưu wallet vào file
    const filePath = path.join(this.walletFileDir, `${name}.json`);
    fs.writeFileSync(filePath, JSON.stringify(wallet));
    return wallet.address; // Trả về hash address thay vì PEM
  }

  // Load wallet từ file
  loadWallet(name) {
    const filePath = path.join(this.walletFileDir, `${name}.json`);
    if (!fs.existsSync(filePath)) {
      throw new Error("Wallet file does not exist");
    }
    const walletData = fs.readFileSync(filePath);
    const wallet = JSON.parse(walletData);
    // Tính address nếu chưa có (wallet cũ)
    if (!wallet.address) {
      wallet.address = publicKeyToAddress(wallet.publicKey);
    }
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
          // Ignore invalid wallet files
        }
      }
    });
  }
  // List tất cả wallet đã load
  listWallets() {
    return Array.from(this.wallets.keys());
  }

  // Lấy hash address (64 hex chars) - dùng cho transactions
  getAddress(name) {
    const wallet = this.wallets.get(name);
    if (!wallet) {
      throw new Error("Wallet not found");
    }
    return wallet.address || publicKeyToAddress(wallet.publicKey);
  }

  // Alias cho getAddress (backward compatible)
  getPublicKey(name) {
    return this.getAddress(name);
  }

  // Lấy PEM public key (để verify signature)
  getPEMPublicKey(name) {
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

  // Import wallet từ private key
  importWallet(name, privateKeyPEM) {
    if (this.wallets.has(name)) {
      throw new Error("Wallet with this name already exists");
    }

    // Validate và extract public key từ private key
    const crypto = require("crypto");
    try {
      const privateKey = crypto.createPrivateKey(privateKeyPEM);
      const publicKey = crypto.createPublicKey(privateKey);

      const publicKeyPEM = publicKey.export({ type: "spki", format: "pem" });
      const address = publicKeyToAddress(publicKeyPEM);

      const wallet = {
        publicKey: publicKeyPEM,
        privateKey: privateKeyPEM,
        address: address,
      };

      this.wallets.set(name, wallet);

      // Lưu wallet vào file
      const filePath = path.join(this.walletFileDir, `${name}.json`);
      fs.writeFileSync(filePath, JSON.stringify(wallet));

      return address;
    } catch (err) {
      throw new Error("Invalid private key format");
    }
  }

  // Delete wallet
  deleteWallet(name) {
    if (!this.wallets.has(name)) {
      throw new Error("Wallet not found");
    }

    this.wallets.delete(name);
    const filePath = path.join(this.walletFileDir, `${name}.json`);
    if (fs.existsSync(filePath)) {
      fs.unlinkSync(filePath);
    }
    return true;
  }
}
module.exports = { WalletManager };
