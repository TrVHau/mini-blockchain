//! CLI — lệnh xem chain: blockchain, block, latest, validate, stats, tx,
//! mempool, fee, reset.

use crate::node::NodeHandle;
use crate::util::{self, *};

pub(super) async fn cmd_blockchain(node: &NodeHandle) {
    let n = node.lock().await;
    println!(
        "\n{CYAN}⛓ Blockchain ({} blocks){RESET}\n",
        n.blockchain.chain.len()
    );
    for block in &n.blockchain.chain {
        println!("{block}");
    }
}

pub(super) async fn cmd_block(node: &NodeHandle, query: &str) {
    let n = node.lock().await;

    // Thử index trước
    if let Ok(index) = query.parse::<usize>() {
        if let Some(block) = n.blockchain.get_block(index) {
            println!("\n{block}");
            return;
        }
    }
    // Thử hash: đầy đủ trước, rồi prefix
    if query.bytes().all(|b| b.is_ascii_hexdigit()) && !query.is_empty() {
        let lower = query.to_lowercase();
        if let Some(block) = n.blockchain.get_block_by_hash(&lower) {
            println!("\n{block}");
            return;
        }
        let matches: Vec<&crate::blockchain::block::Block> = n
            .blockchain
            .chain
            .iter()
            .filter(|b| b.hash.to_lowercase().starts_with(&lower))
            .collect();
        match matches.len() {
            1 => println!("\n{}", matches[0]),
            n if n > 1 => {
                println!(
                    "{}",
                    util::warning(&format!("Multiple blocks match \"{query}\":"))
                );
                for b in matches {
                    println!("  #{}: {}", b.index, util::prefix(&b.hash, 20));
                }
            }
            _ => println!("{}", util::error(&format!("Block not found: \"{query}\""))),
        }
    } else {
        println!(
            "{}",
            util::error("Invalid query. Use block index or hash prefix.")
        );
    }
}

pub(super) async fn cmd_latest(node: &NodeHandle) {
    let n = node.lock().await;
    println!("\n{}", n.blockchain.get_latest_block());
}

pub(super) async fn cmd_validate(node: &NodeHandle) {
    let n = node.lock().await;
    if n.blockchain.is_chain_valid() {
        println!("{}", util::success("Blockchain is valid"));
    } else {
        println!("{}", util::error("Blockchain is invalid!"));
    }
}

pub(super) async fn cmd_stats(node: &NodeHandle) {
    let n = node.lock().await;
    let stats = n.blockchain.get_stats();
    let fee = n.blockchain.estimate_fee();
    let reward = n.blockchain.get_block_reward();
    let valid = n.blockchain.is_chain_valid();

    println!("\n{CYAN}⛓ Blockchain Statistics{RESET}");
    println!("{}", util::divider(29));
    println!(
        "  Height:          {YELLOW}{}{RESET} blocks",
        stats.total_blocks
    );
    println!("  Difficulty:      {YELLOW}{}{RESET}", stats.difficulty);
    println!("  Pending TX:      {YELLOW}{}{RESET}", stats.mempool_size);
    println!(
        "  Total TX:        {YELLOW}{}{RESET}",
        stats.total_transactions
    );
    println!(
        "  Total Coins:     {GREEN}{}{RESET} coins",
        util::fmt_micro_i(stats.total_coins)
    );
    println!(
        "  Block Reward:    {GREEN}{}{RESET} coins",
        util::fmt_micro(reward)
    );
    println!(
        "  Estimated Fee:   {GREEN}{}{RESET} coins",
        util::fmt_micro(fee)
    );
    println!(
        "  Avg Block Time:  {YELLOW}{}{RESET}s",
        stats.avg_block_time
    );
    println!("  Spent TX:        {DIM}{}{RESET}", stats.spent_tx_count);
    println!(
        "  Valid:           {}",
        if valid {
            format!("{GREEN}✓{RESET}")
        } else {
            format!("{RED}✗{RESET}")
        }
    );
    println!();
}

/// Merkle proof cho tx trong block: `proof <block_index> <txid>`
pub(super) async fn cmd_proof(node: &NodeHandle, args: &[&str]) {
    let (Some(index_str), Some(txid)) = (args.first(), args.get(1)) else {
        println!("{}", util::error("Usage: proof <block_index> <txid>"));
        return;
    };
    let Ok(index) = index_str.parse::<usize>() else {
        println!("{}", util::error("Usage: proof <block_index> <txid>"));
        return;
    };

    let n = node.lock().await;
    let Some(block) = n.blockchain.get_block(index) else {
        println!("{}", util::error(&format!("Block not found: {index}")));
        return;
    };
    // Txid có thể là prefix — resolve như cmd_tx
    let Some(tx) = block.transactions.iter().find(|t| {
        t.txid.as_deref().is_some_and(|id| {
            id.starts_with(txid) || id.to_lowercase().starts_with(&txid.to_lowercase())
        })
    }) else {
        println!(
            "{}",
            util::error(&format!("Transaction not in block #{index}: {txid}"))
        );
        return;
    };
    let leaf = tx.txid.clone().unwrap_or_default();

    let verified = block.verify_transaction(&leaf);
    println!("\n{CYAN}📜 Merkle Proof{RESET}");
    println!("{}", util::divider(35));
    println!("  Block:       {CYAN}#{index}{RESET}");
    println!("  TxID:        {YELLOW}{}{RESET}", util::prefix(&leaf, 20));
    println!(
        "  MerkleRoot:  {DIM}{}{RESET}",
        util::prefix(block.merkle_root.as_deref().unwrap_or(""), 20)
    );
    println!(
        "  Verified:    {}",
        if verified {
            format!("{GREEN}✓ belongs to block{RESET}")
        } else {
            format!("{RED}✗ NOT in block{RESET}")
        }
    );
    println!();
}

pub(super) async fn cmd_tx(node: &NodeHandle, query: &str) {
    let n = node.lock().await;

    // Exact match trước, rồi prefix
    let result = n.blockchain.get_transaction(query).or_else(|| {
        if !query.bytes().all(|b| b.is_ascii_hexdigit()) || query.is_empty() {
            return None;
        }
        let lower = query.to_lowercase();
        for block in &n.blockchain.chain {
            if let Some(tx) = block.transactions.iter().find(|t| {
                t.txid
                    .as_deref()
                    .is_some_and(|id| id.to_lowercase().starts_with(&lower))
            }) {
                return n.blockchain.get_transaction(tx.txid.as_deref().unwrap());
            }
        }
        None
    });

    match result {
        None => println!(
            "{}",
            util::error(&format!("Transaction not found: {query}"))
        ),
        Some(info) => {
            let confirmed = n
                .blockchain
                .is_confirmed(info.transaction.txid.as_deref().unwrap_or(""));
            let conf_color = if info.confirmations >= 6 {
                GREEN
            } else {
                YELLOW
            };
            println!("\n{CYAN}📜 Transaction Details{RESET}");
            println!("{}", util::divider(29));
            println!(
                "  TxID:          {YELLOW}{}{RESET}",
                info.transaction.txid.as_deref().unwrap_or("")
            );
            println!("  Block:         {CYAN}#{}{RESET}", info.block_index);
            println!(
                "  From:          {}",
                util::prefix(&info.transaction.from, 16)
            );
            println!(
                "  To:            {}",
                util::prefix(&info.transaction.to, 16)
            );
            println!(
                "  Amount:        {GREEN}{}{RESET} coins",
                util::fmt_micro(info.transaction.amount)
            );
            println!(
                "  Fee:           {YELLOW}{}{RESET} coins",
                util::fmt_micro(info.transaction.fee)
            );
            println!("  Confirmations: {conf_color}{}{RESET}", info.confirmations);
            println!(
                "  Status:        {}",
                if confirmed {
                    format!("{GREEN}Confirmed ✓{RESET}")
                } else {
                    format!("{YELLOW}Pending{RESET}")
                }
            );
            println!();
        }
    }
}

pub(super) async fn cmd_mempool(node: &NodeHandle) {
    let n = node.lock().await;
    let pending = n.blockchain.get_pending_transactions();
    if pending.is_empty() {
        println!("{}", util::info("Mempool is empty"));
        return;
    }
    println!("\n{CYAN}📜 Mempool ({} pending){RESET}", pending.len());
    println!("{}", util::divider(29));
    for (i, tx) in pending.iter().enumerate() {
        println!(
            "  {}. {} → {} | {GREEN}{}{RESET} coins | fee: {}",
            i + 1,
            util::prefix(&tx.from, 12),
            util::prefix(&tx.to, 12),
            util::fmt_micro(tx.amount),
            util::fmt_micro(tx.fee)
        );
    }
    println!();
}

pub(super) async fn cmd_fee(node: &NodeHandle) {
    let n = node.lock().await;
    let fee = n.blockchain.estimate_fee();
    println!(
        "{}",
        util::info(&format!(
            "Estimated fee: {GREEN}{}{RESET} coins",
            util::fmt_micro(fee)
        ))
    );
}

pub(super) async fn cmd_reset(node: &NodeHandle) {
    print!("Are you sure? This will delete all blocks and transactions [y/N]: ");
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    if input.trim().eq_ignore_ascii_case("y") {
        let mut n = node.lock().await;
        n.reset();
        println!("{}", util::success("Blockchain has been reset to genesis"));
    } else {
        println!("{}", util::info("Reset cancelled"));
    }
}
