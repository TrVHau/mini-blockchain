const crypto = require("crypto");

/**
 * Chuyển public key PEM thành address ngắn gọn (hash)
 */
function publicKeyToAddress(publicKey) {
  return crypto.createHash("sha256").update(publicKey).digest("hex");
}

/**
 * Rút gọn address để hiển thị (lấy đầu và cuối)
 */
function shortenAddress(address, prefixLength = 8, suffixLength = 6) {
  if (!address || address === "COINBASE" || address === "SYSTEM") {
    return address;
  }

  // Nếu là PEM key, chuyển sang hash trước
  if (address.includes("BEGIN PUBLIC KEY")) {
    address = publicKeyToAddress(address);
  }

  if (address.length <= prefixLength + suffixLength + 3) {
    return address;
  }

  return `${address.substring(0, prefixLength)}...${address.substring(
    address.length - suffixLength
  )}`;
}

module.exports = {
  publicKeyToAddress,
  shortenAddress,
};
