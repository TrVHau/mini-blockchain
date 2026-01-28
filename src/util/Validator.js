// Validator utility class for common validation logic

class Validator {
  /**
   * Validate port number
   * @param {number|string} port - Port number to validate
   * @returns {number} Valid port number
   * @throws {Error} If port is invalid
   */
  static validatePort(port) {
    const portNum = typeof port === "string" ? parseInt(port) : port;

    if (!Number.isInteger(portNum)) {
      throw new Error("Port must be a valid integer");
    }

    if (portNum < 1 || portNum > 65535) {
      throw new Error("Port must be between 1 and 65535");
    }

    return portNum;
  }

  /**
   * Check if host is localhost
   * @param {string} host - Host to check
   * @returns {boolean} True if host is localhost
   */
  static isLocalhost(host) {
    const localhostValues = ["localhost", "127.0.0.1", "0.0.0.0", "::1"];
    return localhostValues.includes(host.toLowerCase());
  }

  /**
   * Validate WebSocket address format
   * @param {string} address - Address to validate (e.g., "ws://localhost:3000")
   * @returns {boolean} True if address is valid
   */
  static isValidWebSocketAddress(address) {
    if (typeof address !== "string") return false;
    return /^wss?:\/\/.+:\d+$/.test(address);
  }

  /**
   * Parse WebSocket address into components
   * @param {string} address - Address to parse
   * @returns {{protocol: string, host: string, port: number} | null}
   */
  static parseWebSocketAddress(address) {
    const match = address.match(/^(wss?):\/\/(.+):(\d+)$/);
    if (!match) return null;

    return {
      protocol: match[1],
      host: match[2],
      port: parseInt(match[3]),
    };
  }

  /**
   * Validate difficulty level
   * @param {number} difficulty - Difficulty level to validate
   * @param {number} min - Minimum difficulty
   * @param {number} max - Maximum difficulty
   * @returns {number} Valid difficulty
   * @throws {Error} If difficulty is invalid
   */
  static validateDifficulty(difficulty, min, max) {
    const diff =
      typeof difficulty === "string" ? parseInt(difficulty) : difficulty;

    if (!Number.isInteger(diff)) {
      throw new Error("Difficulty must be a valid integer");
    }

    if (diff < min || diff > max) {
      throw new Error(`Difficulty must be between ${min} and ${max}`);
    }

    return diff;
  }

  /**
   * Validate transaction amount
   * @param {number} amount - Amount to validate
   * @returns {number} Valid amount
   * @throws {Error} If amount is invalid
   */
  static validateAmount(amount) {
    if (typeof amount !== "number") {
      throw new Error("Amount must be a number");
    }

    if (amount <= 0) {
      throw new Error("Amount must be greater than 0");
    }

    if (!Number.isFinite(amount)) {
      throw new Error("Amount must be a finite number");
    }

    return amount;
  }

  /**
   * Validate wallet name
   * @param {string} name - Wallet name to validate
   * @returns {string} Valid wallet name
   * @throws {Error} If name is invalid
   */
  static validateWalletName(name) {
    if (typeof name !== "string" || name.trim().length === 0) {
      throw new Error("Wallet name must be a non-empty string");
    }

    // Không cho phép ký tự đặc biệt (chỉ alphanumeric, underscore, dash)
    if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
      throw new Error(
        "Wallet name can only contain letters, numbers, underscore, and dash"
      );
    }

    return name.trim();
  }
}

module.exports = Validator;
