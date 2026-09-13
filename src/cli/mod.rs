//! CLI REPL — port từ src/cli/*. Tương đương vorpal: rustyline + dispatch lệnh.
//! Handlers chia theo nhóm lệnh ở các file con: network, wallets, mining,
//! transfer, chain_view.

mod chain_view;
mod mining;
mod network;
mod transfer;
mod wallets;

use std::sync::Arc;

use crate::node::NodeHandle;
use crate::p2p::P2P;
use crate::util::{self, *};

pub struct Options {
    pub port: Option<u16>,
    pub node_id: String,
    pub connect: Option<String>,
    pub auto_start: bool,
    pub rest_port: Option<u16>,
}

/// Chạy REPL. Trả về khi user gõ exit / Ctrl-C / Ctrl-D (đã cleanup + save).
pub async fn run(node: NodeHandle, p2p: Arc<P2P>, options: &Options) {
    // Banner
    println!("{}", util::banner());
    {
        let n = node.lock().await;
        println!("  {DIM}Node: {CYAN}{}{RESET}", n.node_id);
        println!(
            "  {DIM}Chain: {YELLOW}{}{RESET} blocks\n",
            n.blockchain.chain.len()
        );
    }

    // Auto-start server
    if options.auto_start {
        if let Some(port) = options.port {
            p2p.start_server(port).await;
        }
    }

    // Sync watchdog: retry khi sync stale (thay cho setTimeout đệ quy của JS)
    tokio::spawn(crate::p2p::sync::sync_watchdog(
        node.clone(),
        p2p.peers.clone(),
    ));

    // Auto-connect
    if let Some(connect) = &options.connect {
        if let Some((host, port)) = connect.split_once(':') {
            if let Ok(port) = port.parse::<u16>() {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                p2p.connect_to_peer(host, port).await;
            }
        }
    }

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("Failed to init REPL: {e}");
            return;
        }
    };

    loop {
        let prompt = format!("{CYAN}⛓ {} {YELLOW}➜{RESET} ", options.node_id);
        match rl.readline(&prompt) {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);
                if !handle_command(&node, &p2p, line.trim()).await {
                    break;
                }
            }
            Err(_) => {
                // Ctrl-C / Ctrl-D / lỗi terminal: thoát sạch như SIGINT handler của JS
                break;
            }
        }
    }

    // Cleanup
    {
        let n = node.lock().await;
        n.storage.save_blockchain(&n.blockchain.chain);
    }
    p2p.close();
    println!("{}", util::success("Goodbye!"));
}

/// Xử lý 1 lệnh. Trả về false để thoát REPL.
async fn handle_command(node: &NodeHandle, p2p: &Arc<P2P>, line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap_or_default();
    let args: Vec<&str> = parts.collect();

    match cmd {
        "help" | "?" => print_help(),
        "exit" | "quit" | "q" => return false,

        // ---- Network ----
        "open" | "o" => network::cmd_open(p2p, &args).await,
        "connect" | "c" => network::cmd_connect(p2p, &args).await,
        "peers" | "p" => network::cmd_peers(p2p),
        "status" | "s" => network::cmd_status(node, p2p).await,
        "sync" => network::cmd_sync(node, p2p).await,
        "close" => p2p.close_server(),
        "disconnect" | "dc" => network::cmd_disconnect(p2p, &args),

        // ---- Wallet ----
        "wallet-create" | "wc" => wallets::cmd_wallet_create(node, &args).await,
        "wallets" | "wl" => wallets::cmd_wallets(node, &args).await,
        "balance" | "bal" => wallets::cmd_balance(node, &args).await,
        "address" | "addr" => wallets::cmd_address(node, args.first().copied().unwrap_or("")).await,
        "history" | "h" => wallets::cmd_history(node, args.first().copied().unwrap_or("")).await,
        "export" => wallets::cmd_export(node, args.first().copied().unwrap_or("")).await,
        "import" => wallets::cmd_import(node, args.first().copied().unwrap_or("")).await,
        "wallet-delete" | "wd" => {
            wallets::cmd_wallet_delete(node, args.first().copied().unwrap_or("")).await
        }

        // ---- Transaction / Mining ----
        "send" => transfer::cmd_send(node, p2p, &args).await,
        "mine" | "m" => mining::cmd_mine(node, p2p, args.first().copied().unwrap_or("")).await,
        "automine" | "am" => mining::cmd_automine(node, p2p, &args).await,
        "stopautomine" | "sam" => mining::cmd_stop_automine(node).await,

        // ---- Blockchain ----
        "blockchain" | "bc" => chain_view::cmd_blockchain(node).await,
        "block" | "b" => chain_view::cmd_block(node, args.first().copied().unwrap_or("")).await,
        "latest" | "l" => chain_view::cmd_latest(node).await,
        "validate" | "v" => chain_view::cmd_validate(node).await,
        "stats" => chain_view::cmd_stats(node).await,
        "tx" => chain_view::cmd_tx(node, args.first().copied().unwrap_or("")).await,
        "mempool" | "mp" => chain_view::cmd_mempool(node).await,
        "fee" => chain_view::cmd_fee(node).await,
        "reset" => chain_view::cmd_reset(node).await,

        other => println!(
            "{}",
            util::error(&format!("Unknown command: {other}. Type 'help'."))
        ),
    }
    true
}

fn print_help() {
    println!("\n{CYAN}⛓ Commands:{RESET}");
    println!("  {BRIGHT}Network:{RESET}   open <port> | connect <host> <port> | peers | status | sync | close | disconnect [idx]");
    println!("  {BRIGHT}Wallet:{RESET}    wallet-create <name> | wallets [all] | balance <name> | address <name> | history <name>");
    println!("             export <name> | import <name> | wallet-delete <name>");
    println!(
        "  {BRIGHT}Mining:{RESET}    mine <wallet> | automine <wallet> [interval] | stopautomine"
    );
    println!("  {BRIGHT}Transfers:{RESET} send <from> <to> <amount> [fee]");
    println!("  {BRIGHT}Chain:{RESET}     blockchain | block <idx|hash> | latest | validate | stats | tx <txid> | mempool | fee | reset");
    println!("  {BRIGHT}Misc:{RESET}      help | exit\n");
}

// ---- Address resolution (3 tầng như JS) ----

async fn resolve_address(node: &NodeHandle, query: &str) -> Result<String, String> {
    node.lock().await.resolve_address(query)
}
