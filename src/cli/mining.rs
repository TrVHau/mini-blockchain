//! CLI — lệnh mining: mine, automine, stopautomine.

use std::sync::Arc;

use crate::node::NodeHandle;
use crate::p2p::{messages, P2P};
use crate::util::{self, *};

use super::resolve_address;

pub(super) async fn cmd_mine(node: &NodeHandle, p2p: &Arc<P2P>, wallet: &str) {
    let miner_address = match resolve_address(node, wallet).await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", util::error(&e));
            return;
        }
    };
    let display = util::shorten_address(&miner_address);
    println!("{}", util::info(&format!("Mining block for {display}...")));

    // PoW chạy ngoài lock — node vẫn phản hồi các lệnh khác trong lúc mine
    let Some(block) = crate::node::mine_block(node, &miner_address).await else {
        println!(
            "{}",
            util::warning("Chain changed while mining — transactions returned to mempool")
        );
        return;
    };

    let lines = vec![
        util::key_value("Block", &format!("#{CYAN}{}{RESET}", block.index)),
        util::key_value(
            "Transactions",
            &format!("{YELLOW}{}{RESET}", block.transactions.len()),
        ),
        util::key_value("Nonce", &format!("{DIM}{}{RESET}", block.nonce)),
        util::key_value(
            "Hash",
            &format!("{DIM}{}{RESET}", util::prefix(&block.hash, 16)),
        ),
    ];
    println!("\n{}", util::box_lines(&lines, "⛏ Block Mined", 45));
    let reward = block.coinbase_tx.as_ref().map(|c| c.amount).unwrap_or(0);
    println!(
        "{}",
        util::success(&format!("Reward: {} coins", util::fmt_micro(reward)))
    );

    p2p.broadcast(&messages::new_block(&block));
    println!(
        "{}\n",
        util::info(&format!("Broadcasted to {} peer(s)", p2p.get_peers().len()))
    );
}

pub(super) async fn cmd_automine(node: &NodeHandle, p2p: &Arc<P2P>, args: &[&str]) {
    // Toggle: đang chạy -> stop (thả lock trước khi stop lock lại)
    if node.lock().await.auto_mine.is_some() {
        cmd_stop_automine(node).await;
        return;
    }

    let mut n = node.lock().await;

    let Some(wallet) = args.first() else {
        println!("{}", util::error("Usage: automine <wallet> [interval]"));
        return;
    };
    let miner_address = match n.wallets.get_address(wallet) {
        Ok(a) => a,
        Err(_) if util::is_valid_address(wallet) => wallet.to_string(),
        Err(_) => {
            println!("{}", util::error(&format!("Wallet not found: {wallet}")));
            return;
        }
    };
    let interval = args
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);
    let display = util::shorten_address(&miner_address);
    println!(
        "{}",
        util::success(&format!(
            "Auto-mine started for {display} (every {interval}s)"
        ))
    );
    println!("{}\n", util::info("Run 'automine' again to stop"));

    n.auto_mine = Some((miner_address, interval));
    let node_clone = node.clone();
    let p2p_clone = p2p.clone();
    n.auto_mine_task = Some(tokio::spawn(auto_mine_task(node_clone, p2p_clone)));
}

async fn auto_mine_task(node: NodeHandle, p2p: Arc<P2P>) {
    loop {
        let interval = {
            let n = node.lock().await;
            let Some((_, interval)) = n.auto_mine.clone() else {
                break;
            };
            interval
        };
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        let block = {
            let n = node.lock().await;
            if n.auto_mine.is_none() {
                break;
            }
            if n.blockchain.mempool.is_empty() {
                continue;
            }
            println!(
                "{}",
                util::info(&format!(
                    "Auto-mining {} pending tx(s)...",
                    n.blockchain.mempool.len()
                ))
            );
            let miner = n.auto_mine.as_ref().unwrap().0.clone();
            drop(n);
            match crate::node::mine_block(&node, &miner).await {
                Some(b) => b,
                None => continue, // chain đổi giữa chừng — vòng sau thử lại
            }
        };
        let reward = block.coinbase_tx.as_ref().map(|c| c.amount).unwrap_or(0);
        println!(
            "{}",
            util::success(&format!(
                "Block #{} mined! Reward: {} coins",
                block.index,
                util::fmt_micro(reward)
            ))
        );
        p2p.broadcast(&messages::new_block(&block));
    }
}

pub(super) async fn cmd_stop_automine(node: &NodeHandle) {
    let mut n = node.lock().await;
    if let Some(task) = n.auto_mine_task.take() {
        task.abort();
        n.auto_mine = None;
        println!("{}", util::success("Auto-mine stopped"));
    } else {
        println!("{}", util::info("Auto-mine is not running"));
    }
}
