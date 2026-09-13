//! REST API (axum) — query chain/ví/mempool, tạo ví, gửi transaction.
//! Bind 127.0.0.1 only: learning project, không expose private key qua GET.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::blockchain::transaction::Transaction;
use crate::merkle;
use crate::node::NodeHandle;
use crate::p2p::{messages, P2P};
use crate::util;

type ApiState = (NodeHandle, Arc<P2P>);

pub async fn serve(node: NodeHandle, p2p: Arc<P2P>, port: u16) {
    let app = Router::new()
        .route("/info", get(info))
        .route("/stats", get(stats))
        .route("/chain", get(chain))
        .route("/blocks/{query}", get(block))
        .route("/blocks/{index}/proof/{txid}", get(merkle_proof))
        .route("/tx/{txid}", get(tx))
        .route("/mempool", get(mempool))
        .route("/fee", get(fee))
        .route("/balance/{address}", get(balance))
        .route("/wallets", get(wallets).post(create_wallet))
        .route("/transactions", post(create_transaction))
        .with_state((node, p2p));

    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[API] ✗ Cannot bind port {port}: {e}");
            return;
        }
    };
    println!("[API] ✓ REST API running on http://127.0.0.1:{port}");
    let _ = axum::serve(listener, app).await;
}

fn ok(data: Value) -> Response {
    (StatusCode::OK, Json(data)).into_response()
}

fn err(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}

// ---- GET handlers ----

async fn info(State((node, _)): State<ApiState>) -> Response {
    let info = node.lock().await.get_node_info();
    ok(serde_json::to_value(info).unwrap_or_default())
}

async fn stats(State((node, _)): State<ApiState>) -> Response {
    let n = node.lock().await;
    let stats = n.blockchain.get_stats();
    ok(serde_json::to_value(stats).unwrap_or_default())
}

async fn chain(State((node, _)): State<ApiState>) -> Response {
    let n = node.lock().await;
    ok(json!({ "height": n.blockchain.chain.len(), "blocks": n.blockchain.chain }))
}

async fn block(State((node, _)): State<ApiState>, Path(query): Path<String>) -> Response {
    let n = node.lock().await;
    let found = query
        .parse::<usize>()
        .ok()
        .and_then(|i| n.blockchain.get_block(i))
        .or_else(|| n.blockchain.get_block_by_hash(&query.to_lowercase()));
    match found {
        Some(b) => ok(serde_json::to_value(b).unwrap_or_default()),
        None => err(StatusCode::NOT_FOUND, &format!("Block not found: {query}")),
    }
}

async fn tx(State((node, _)): State<ApiState>, Path(txid): Path<String>) -> Response {
    let n = node.lock().await;
    match n.blockchain.get_transaction(&txid) {
        Some(info) => ok(serde_json::to_value(info).unwrap_or_default()),
        None => err(
            StatusCode::NOT_FOUND,
            &format!("Transaction not found: {txid}"),
        ),
    }
}

async fn mempool(State((node, _)): State<ApiState>) -> Response {
    let n = node.lock().await;
    let pending: Vec<Transaction> = n
        .blockchain
        .get_pending_transactions()
        .into_iter()
        .cloned()
        .collect();
    ok(json!({ "count": pending.len(), "transactions": pending }))
}

async fn fee(State((node, _)): State<ApiState>) -> Response {
    let n = node.lock().await;
    let fee = n.blockchain.estimate_fee();
    ok(json!({ "feeMicro": fee, "feeCoins": util::fmt_micro(fee) }))
}

async fn balance(State((node, _)): State<ApiState>, Path(address): Path<String>) -> Response {
    let n = node.lock().await;
    match n.resolve_address(&address) {
        Ok(addr) => {
            let balance = n.blockchain.get_balance(&addr);
            ok(json!({
                "address": addr,
                "balanceMicro": balance,
                "balanceCoins": util::fmt_micro_i(balance),
            }))
        }
        Err(e) => err(StatusCode::NOT_FOUND, &e),
    }
}

async fn wallets(State((node, _)): State<ApiState>) -> Response {
    let n = node.lock().await;
    let wallets: Vec<Value> = n
        .wallets
        .list_wallets()
        .into_iter()
        .map(|name| {
            let address = n.wallets.get_address(&name).unwrap_or_default();
            json!({ "name": name, "address": address })
        })
        .collect();
    ok(json!({ "count": wallets.len(), "wallets": wallets }))
}

/// Merkle proof cho tx trong block (dùng chung leaf list với merkle root)
async fn merkle_proof(
    State((node, _)): State<ApiState>,
    Path((index, txid)): Path<(usize, String)>,
) -> Response {
    let n = node.lock().await;
    let Some(block) = n.blockchain.get_block(index) else {
        return err(StatusCode::NOT_FOUND, &format!("Block not found: {index}"));
    };
    let Some(tx) = block
        .transactions
        .iter()
        .find(|t| t.txid.as_deref() == Some(txid.as_str()))
    else {
        return err(
            StatusCode::NOT_FOUND,
            &format!("Transaction not in block #{index}: {txid}"),
        );
    };
    let leaf = tx.txid.clone().unwrap_or_default();
    let leaves = block.merkle_leaves();
    let Some(leaf_index) = leaves.iter().position(|h| h == &leaf) else {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "txid not found in merkle leaves",
        );
    };
    let Some(proof) = merkle::get_proof(&leaves, leaf_index) else {
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cannot build merkle proof",
        );
    };
    let root = block.merkle_root.as_deref().unwrap_or("");
    ok(json!({
        "txid": txid,
        "blockIndex": index,
        "merkleRoot": root,
        "proof": proof,
        "verified": merkle::verify_proof(&leaf, &proof, root),
    }))
}

// ---- POST handlers ----

#[derive(Deserialize)]
struct CreateWalletRequest {
    name: String,
}

/// Tạo ví — private key chỉ trả về một lần trong response này
async fn create_wallet(
    State((node, _)): State<ApiState>,
    Json(req): Json<CreateWalletRequest>,
) -> Response {
    let mut n = node.lock().await;
    match n.wallets.create_wallet(&req.name) {
        Ok(address) => {
            let private_key = n.wallets.get_private_key(&req.name).unwrap_or_default();
            ok(json!({
                "name": req.name,
                "address": address,
                "privateKey": private_key,
            }))
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

#[derive(Deserialize)]
struct CreateTransactionRequest {
    from: String,
    to: String,
    /// Số coins thập phân, ví dụ "12.5"
    amount: String,
    #[serde(default)]
    fee: String,
}

/// Gửi tx: `from` phải là wallet local (cần private key để ký)
async fn create_transaction(
    State((node, p2p)): State<ApiState>,
    Json(req): Json<CreateTransactionRequest>,
) -> Response {
    let mut n = node.lock().await;

    let from_address = match n.wallets.get_address(&req.from) {
        Ok(a) => a,
        Err(_) => {
            return err(
                StatusCode::BAD_REQUEST,
                &format!("Sender wallet not found: {}", req.from),
            )
        }
    };
    let (sk, pk) = match (
        n.wallets.get_private_key(&req.from),
        n.wallets.get_public_key_hex(&req.from),
    ) {
        (Ok(sk), Ok(pk)) => (sk, pk),
        _ => return err(StatusCode::INTERNAL_SERVER_ERROR, "Cannot load sender keys"),
    };
    let to_address = match n.resolve_address(&req.to) {
        Ok(a) => a,
        Err(e) => return err(StatusCode::BAD_REQUEST, &e),
    };
    let amount = match util::coins_str_to_micro(&req.amount) {
        Ok(a) => a,
        Err(e) => return err(StatusCode::BAD_REQUEST, &e),
    };
    let fee = match util::coins_str_to_micro(&req.fee) {
        Ok(f) => f,
        Err(e) => return err(StatusCode::BAD_REQUEST, &e),
    };

    let mut tx = Transaction::new(&from_address, &to_address, amount, fee);
    if let Err(e) = tx.sign(&sk, &pk) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, &e);
    }
    if let Err(e) = n.blockchain.add_transaction(&tx) {
        return err(StatusCode::BAD_REQUEST, &e);
    }

    let txid = tx.txid.clone().unwrap_or_default();
    p2p.broadcast(&messages::transaction(&tx));
    ok(json!({
        "txid": txid,
        "from": from_address,
        "to": to_address,
        "amountMicro": amount,
        "feeMicro": fee,
        "broadcastedTo": p2p.get_peers().len(),
    }))
}
