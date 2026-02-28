const fs = require("fs");
const path = require("path");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("STORAGE");

/**
 * Storage - Lưu trữ blockchain và wallet cho mỗi node
 */
class Storage {
  constructor(nodeId = "default") {
    this.nodeId = nodeId;
    this.dataDir = path.join(process.cwd(), "data", "nodes", nodeId);
    this._ensureDir();
  }

  _ensureDir() {
    if (!fs.existsSync(this.dataDir)) {
      fs.mkdirSync(this.dataDir, { recursive: true });
    }
  }

  _path(filename) {
    return path.join(this.dataDir, filename);
  }

  // Blockchain
  saveBlockchain(chain) {
    try {
      fs.writeFileSync(this._path("blockchain.json"), JSON.stringify(chain));
      return true;
    } catch (err) {
      logger.error("Error saving blockchain:", err.message);
      return false;
    }
  }

  loadBlockchain() {
    try {
      const filePath = this._path("blockchain.json");
      if (fs.existsSync(filePath)) {
        return JSON.parse(fs.readFileSync(filePath, "utf8"));
      }
      return null;
    } catch (err) {
      logger.error("Error loading blockchain:", err.message);
      return null;
    }
  }

  getDataDir() {
    return this.dataDir;
  }
}

module.exports = Storage;
