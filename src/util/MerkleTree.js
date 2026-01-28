const crypto = require("crypto");

/**
 * Merkle Tree - Cấu trúc dữ liệu để verify transactions hiệu quả
 */
class MerkleTree {
  /**
   * Tính hash SHA256
   */
  static hash(data) {
    return crypto.createHash("sha256").update(data).digest("hex");
  }

  /**
   * Tính hash của 2 nodes
   */
  static hashPair(left, right) {
    return this.hash(left + right);
  }

  /**
   * Tính Merkle Root từ danh sách transaction hashes
   * @param {string[]} txHashes - Mảng các transaction hash
   * @returns {string} Merkle root hash
   */
  static calculateRoot(txHashes) {
    if (!txHashes || txHashes.length === 0) {
      return this.hash("empty");
    }

    if (txHashes.length === 1) {
      return txHashes[0];
    }

    // Copy array để không modify original
    let level = [...txHashes];

    // Nếu số lẻ, duplicate phần tử cuối
    if (level.length % 2 !== 0) {
      level.push(level[level.length - 1]);
    }

    // Build tree từ dưới lên
    while (level.length > 1) {
      const nextLevel = [];

      for (let i = 0; i < level.length; i += 2) {
        const left = level[i];
        const right = level[i + 1] || left; // Duplicate nếu lẻ
        nextLevel.push(this.hashPair(left, right));
      }

      level = nextLevel;

      // Đảm bảo level có số chẵn (trừ khi chỉ còn 1)
      if (level.length > 1 && level.length % 2 !== 0) {
        level.push(level[level.length - 1]);
      }
    }

    return level[0];
  }

  /**
   * Tạo Merkle Proof cho một transaction
   * @param {string[]} txHashes - Danh sách tất cả tx hashes
   * @param {number} index - Vị trí của tx cần proof
   * @returns {Array} Proof path
   */
  static getProof(txHashes, index) {
    if (!txHashes || txHashes.length === 0 || index >= txHashes.length) {
      return null;
    }

    const proof = [];
    let level = [...txHashes];

    if (level.length % 2 !== 0) {
      level.push(level[level.length - 1]);
    }

    let currentIndex = index;

    while (level.length > 1) {
      const nextLevel = [];

      for (let i = 0; i < level.length; i += 2) {
        const left = level[i];
        const right = level[i + 1] || left;

        // Nếu current index nằm trong cặp này
        if (i === currentIndex || i + 1 === currentIndex) {
          const isLeft = currentIndex % 2 === 0;
          proof.push({
            hash: isLeft ? right : left,
            position: isLeft ? "right" : "left",
          });
        }

        nextLevel.push(this.hashPair(left, right));
      }

      level = nextLevel;
      currentIndex = Math.floor(currentIndex / 2);

      if (level.length > 1 && level.length % 2 !== 0) {
        level.push(level[level.length - 1]);
      }
    }

    return proof;
  }

  /**
   * Verify một transaction thuộc về tree với merkle root
   * @param {string} txHash - Hash của transaction cần verify
   * @param {Array} proof - Merkle proof
   * @param {string} root - Merkle root
   * @returns {boolean}
   */
  static verifyProof(txHash, proof, root) {
    let currentHash = txHash;

    for (const node of proof) {
      if (node.position === "left") {
        currentHash = this.hashPair(node.hash, currentHash);
      } else {
        currentHash = this.hashPair(currentHash, node.hash);
      }
    }

    return currentHash === root;
  }
}

module.exports = MerkleTree;
