const MESSAGE_TYPE = {
  // Block & Chain sync
  NEW_BLOCK: "NEW_BLOCK",
  REQUEST_CHAIN: "REQUEST_CHAIN",
  RECEIVE_CHAIN: "RECEIVE_CHAIN",
  REQUEST_LATEST: "REQUEST_LATEST",
  REQUEST_BLOCKS_FROM: "REQUEST_BLOCKS_FROM", // Request blocks từ index cụ thể
  RECEIVE_BLOCKS: "RECEIVE_BLOCKS", // Nhận partial blocks

  // Transactions
  TRANSACTION: "TRANSACTION",

  // Handshake & Discovery
  HANDSHAKE: "HANDSHAKE", // Node mới gửi thông tin về mình
  HANDSHAKE_ACK: "HANDSHAKE_ACK", // Xác nhận handshake + gửi chain height
  REQUEST_PEERS: "REQUEST_PEERS",
  RECEIVE_PEERS: "RECEIVE_PEERS",

  // Sync status
  SYNC_STATUS: "SYNC_STATUS", // Thông báo trạng thái sync
};

module.exports = MESSAGE_TYPE;
