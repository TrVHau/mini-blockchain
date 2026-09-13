//! Wire messages — port từ src/p2p/Messages.js + message-type.js.
//! Định dạng JSON `{type, data}` giữ nguyên tên type như bản JS.

use serde_json::{json, Value};

use crate::blockchain::block::Block;
use crate::blockchain::transaction::Transaction;
use crate::node::NodeInfo;

pub mod message_type {
    // Block & Chain sync
    pub const NEW_BLOCK: &str = "NEW_BLOCK";
    pub const REQUEST_CHAIN: &str = "REQUEST_CHAIN";
    pub const RECEIVE_CHAIN: &str = "RECEIVE_CHAIN";
    pub const REQUEST_LATEST: &str = "REQUEST_LATEST";
    pub const REQUEST_BLOCKS_FROM: &str = "REQUEST_BLOCKS_FROM";
    pub const RECEIVE_BLOCKS: &str = "RECEIVE_BLOCKS";

    // Transactions
    pub const TRANSACTION: &str = "TRANSACTION";

    // Handshake
    pub const HANDSHAKE: &str = "HANDSHAKE";
    pub const HANDSHAKE_ACK: &str = "HANDSHAKE_ACK";
}

fn wrap(msg_type: &str, data: Value) -> String {
    json!({ "type": msg_type, "data": data }).to_string()
}

pub fn new_block(block: &Block) -> String {
    wrap(message_type::NEW_BLOCK, json!({ "block": block }))
}

pub fn request_chain() -> String {
    wrap(message_type::REQUEST_CHAIN, json!(null))
}

pub fn receive_chain(chain: &[Block]) -> String {
    wrap(message_type::RECEIVE_CHAIN, json!({ "chain": chain }))
}

#[allow(dead_code)] // REQUEST_LATEST được xử lý ở receiver nhưng node này chưa dùng
pub fn request_latest() -> String {
    wrap(message_type::REQUEST_LATEST, json!(null))
}

pub fn request_blocks_from(from_index: usize) -> String {
    wrap(message_type::REQUEST_BLOCKS_FROM, json!({ "fromIndex": from_index }))
}

pub fn receive_blocks(blocks: &[Block], from_index: usize, total_height: usize) -> String {
    wrap(
        message_type::RECEIVE_BLOCKS,
        json!({ "blocks": blocks, "fromIndex": from_index, "totalHeight": total_height }),
    )
}

pub fn transaction(tx: &Transaction) -> String {
    wrap(message_type::TRANSACTION, json!({ "transaction": tx }))
}

pub fn handshake(node_info: &NodeInfo) -> String {
    wrap(message_type::HANDSHAKE, json!(node_info))
}

pub fn handshake_ack(node_info: &NodeInfo) -> String {
    wrap(message_type::HANDSHAKE_ACK, json!(node_info))
}

/// Parse message -> (type, data). Data rỗng nếu message không có data.
pub fn parse(text: &str) -> Result<(String, Value), String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid message format: {e}"))?;
    let msg_type = value
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or("Message missing 'type'")?
        .to_string();
    let data = value.get("data").cloned().unwrap_or(Value::Null);
    Ok((msg_type, data))
}
