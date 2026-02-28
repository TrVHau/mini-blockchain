const Messages = require("./Messages.js");
const BLOCKCHAIN_CONSTANTS = require("../config/constants.js");
const { Logger } = require("../util/Logger.js");

const logger = new Logger("SYNC");

/**
 * SyncManager - Quản lý đồng bộ blockchain giữa các node
 *
 * Giải quyết các vấn đề:
 * 1. Node mới kết nối cần sync toàn bộ chain
 * 2. Partial sync khi chỉ thiếu vài block
 * 3. Retry mechanism khi sync thất bại
 * 4. Tránh sync đồng thời từ nhiều peer
 */
class SyncManager {
  constructor(blockchain) {
    this.blockchain = blockchain;
    this.syncState = {
      isSyncing: false,
      syncingFromPeer: null,
      lastSyncAttempt: 0,
      retryCount: 0,
      pendingBlocks: [],
    };

    // Config
    this.MAX_RETRY = 3;
    this.SYNC_TIMEOUT = 30000; // 30s
    this.SYNC_COOLDOWN = 5000; // 5s between retries
    this.MAX_BLOCKS_PER_REQUEST = 50; // Giới hạn blocks mỗi request
  }

  /**
   * Bắt đầu sync với một peer
   * @param {WebSocket} socket - Peer socket
   * @param {Function} sendMessage - Hàm gửi message
   * @param {number} peerHeight - Chiều cao chain của peer
   */
  startSync(socket, sendMessage, peerHeight) {
    const localHeight = this.blockchain.getLatestBlock().index;

    // Không cần sync nếu chain của mình đã dài hơn hoặc bằng
    if (localHeight >= peerHeight) {
      logger.debug(
        `No sync needed. Local: ${localHeight}, Peer: ${peerHeight}`,
      );
      return false;
    }

    // Đang sync thì không sync lại
    if (this.syncState.isSyncing) {
      const timeSinceLastSync = Date.now() - this.syncState.lastSyncAttempt;
      if (timeSinceLastSync < this.SYNC_TIMEOUT) {
        logger.debug("Already syncing, please wait...");
        return false;
      }
      // Timeout, reset sync state
      logger.warn("Previous sync timed out, retrying...");
      this.resetSyncState();
    }

    // Bắt đầu sync
    this.syncState.isSyncing = true;
    this.syncState.syncingFromPeer = socket;
    this.syncState.lastSyncAttempt = Date.now();

    logger.info(`Starting sync from block ${localHeight} to ${peerHeight}...`);

    // Quyết định sync partial hay full
    const blocksBehind = peerHeight - localHeight;

    if (blocksBehind <= this.MAX_BLOCKS_PER_REQUEST && localHeight > 0) {
      // Partial sync - chỉ lấy những block thiếu
      logger.info(`Requesting ${blocksBehind} missing blocks...`);
      sendMessage(socket, Messages.requestBlocksFrom(localHeight + 1));
    } else {
      // Full sync - lấy toàn bộ chain
      logger.info(
        `Requesting full blockchain (${blocksBehind} blocks behind)...`,
      );
      sendMessage(socket, Messages.requestChain());
    }

    // Set timeout để auto-retry
    this._setSyncTimeout(socket, sendMessage, peerHeight);

    return true;
  }

  /**
   * Xử lý khi nhận được partial blocks
   */
  handleReceiveBlocks(blocks, fromIndex, totalHeight, socket) {
    if (!this.syncState.isSyncing) {
      logger.debug("Received blocks but not syncing, ignoring...");
      return false;
    }

    if (socket !== this.syncState.syncingFromPeer) {
      logger.debug("Received blocks from different peer, ignoring...");
      return false;
    }

    logger.info(`Received ${blocks.length} blocks starting from #${fromIndex}`);

    let addedCount = 0;
    for (const block of blocks) {
      if (this.blockchain.receiveBlock(block)) {
        addedCount++;
      } else {
        logger.warn(`Failed to add block #${block.index}, stopping sync`);
        break;
      }
    }

    logger.info(`Added ${addedCount}/${blocks.length} blocks`);

    // Kiểm tra đã sync xong chưa
    const currentHeight = this.blockchain.getLatestBlock().index;
    if (currentHeight >= totalHeight) {
      this.completSync();
      return true;
    }

    // Còn thiếu blocks, request tiếp
    this.syncState.lastSyncAttempt = Date.now();
    return true;
  }

  /**
   * Xử lý khi nhận full chain
   */
  handleReceiveChain(chain) {
    if (!chain || !Array.isArray(chain)) {
      logger.error("Invalid chain received");
      return false;
    }

    const result = this.blockchain.receiveChain(chain);

    if (result) {
      this.completSync();
    } else {
      this.handleSyncFailure("Chain validation failed");
    }

    return result;
  }

  /**
   * Hoàn thành sync
   */
  completSync() {
    const height = this.blockchain.getLatestBlock().index;
    logger.success(`Sync completed! Chain height: ${height}`);
    this.resetSyncState();
  }

  /**
   * Xử lý khi sync thất bại
   */
  handleSyncFailure(reason) {
    logger.warn(`Sync failed: ${reason}`);

    this.syncState.retryCount++;

    if (this.syncState.retryCount < this.MAX_RETRY) {
      logger.info(
        `Will retry (${this.syncState.retryCount}/${this.MAX_RETRY})...`,
      );
      this.syncState.isSyncing = false;
    } else {
      logger.error("Max retries reached, giving up sync from this peer");
      this.resetSyncState();
    }
  }

  /**
   * Reset trạng thái sync
   */
  resetSyncState() {
    this.syncState = {
      isSyncing: false,
      syncingFromPeer: null,
      lastSyncAttempt: 0,
      retryCount: 0,
      pendingBlocks: [],
    };
  }

  /**
   * Set timeout cho sync
   */
  _setSyncTimeout(socket, sendMessage, peerHeight) {
    setTimeout(() => {
      if (
        this.syncState.isSyncing &&
        this.syncState.syncingFromPeer === socket
      ) {
        logger.warn("Sync timeout, retrying...");
        this.handleSyncFailure("Timeout");

        // Auto retry
        if (this.syncState.retryCount < this.MAX_RETRY) {
          setTimeout(() => {
            this.startSync(socket, sendMessage, peerHeight);
          }, this.SYNC_COOLDOWN);
        }
      }
    }, this.SYNC_TIMEOUT);
  }

  /**
   * Lấy blocks để gửi cho peer (partial sync)
   */
  getBlocksFrom(fromIndex) {
    const chain = this.blockchain.get();
    const blocks = [];

    for (
      let i = fromIndex;
      i < chain.length && blocks.length < this.MAX_BLOCKS_PER_REQUEST;
      i++
    ) {
      blocks.push(chain[i]);
    }

    return {
      blocks,
      fromIndex,
      totalHeight: chain.length - 1,
    };
  }

  /**
   * Lấy trạng thái sync hiện tại
   */
  getStatus() {
    return {
      isSyncing: this.syncState.isSyncing,
      localHeight: this.blockchain.getLatestBlock().index,
      retryCount: this.syncState.retryCount,
      lastSyncAttempt: this.syncState.lastSyncAttempt,
    };
  }

  /**
   * Tạo node info cho handshake
   */
  getNodeInfo() {
    return {
      chainHeight: this.blockchain.getLatestBlock().index,
      latestBlockHash: this.blockchain.getLatestBlock().hash,
      mempoolSize: this.blockchain.mempool.length,
      timestamp: Date.now(),
    };
  }
}

module.exports = SyncManager;
