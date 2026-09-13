//! CLI REPL — port từ src/cli/*. Tương đương vorpal: rustyline + dispatch lệnh.

use std::sync::Arc;

use crate::blockchain::transaction::Transaction;
use crate::node::NodeHandle;
use crate::p2p::{messages, P2P};
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
        "open" | "o" => cmd_open(p2p, &args).await,
        "connect" | "c" => cmd_connect(p2p, &args).await,
        "peers" | "p" => cmd_peers(p2p),
        "status" | "s" => cmd_status(node, p2p).await,
        "sync" => cmd_sync(node, p2p).await,
        "close" => p2p.close_server(),
        "disconnect" | "dc" => cmd_disconnect(p2p, &args),

        // ---- Wallet ----
        "wallet-create" | "wc" => cmd_wallet_create(node, &args).await,
        "wallets" | "wl" => cmd_wallets(node, &args).await,
        "balance" | "bal" => cmd_balance(node, &args).await,
        "address" | "addr" => cmd_address(node, args.first().copied().unwrap_or("")).await,
        "history" | "h" => cmd_history(node, args.first().copied().unwrap_or("")).await,
        "export" => cmd_export(node, args.first().copied().unwrap_or("")).await,
        "import" => cmd_import(node, args.first().copied().unwrap_or("")).await,
        "wallet-delete" | "wd" => {
            cmd_wallet_delete(node, args.first().copied().unwrap_or("")).await
        }

        // ---- Transaction / Mining ----
        "send" => cmd_send(node, p2p, &args).await,
        "mine" | "m" => cmd_mine(node, p2p, args.first().copied().unwrap_or("")).await,
        "automine" | "am" => cmd_automine(node, p2p, &args).await,
        "stopautomine" | "sam" => cmd_stop_automine(node).await,

        // ---- Blockchain ----
        "blockchain" | "bc" => cmd_blockchain(node).await,
        "block" | "b" => cmd_block(node, args.first().copied().unwrap_or("")).await,
        "latest" | "l" => cmd_latest(node).await,
        "validate" | "v" => cmd_validate(node).await,
        "stats" => cmd_stats(node).await,
        "tx" => cmd_tx(node, args.first().copied().unwrap_or("")).await,
        "mempool" | "mp" => cmd_mempool(node).await,
        "fee" => cmd_fee(node).await,
        "reset" => cmd_reset(node).await,

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

// ---- Network commands ----

async fn cmd_open(p2p: &Arc<P2P>, args: &[&str]) {
    match args.first().map(|p| util::validate_port(p)) {
        Some(Ok(port)) => p2p.start_server(port).await,
        _ => println!("{}", util::error("Usage: open <port> (1-65535)")),
    }
}

async fn cmd_connect(p2p: &Arc<P2P>, args: &[&str]) {
    if let (Some(host), Some(port)) = (args.first(), args.get(1)) {
        match util::validate_port(port) {
            Ok(port) => {
                println!("{}", util::info(&format!("Connecting to {host}:{port}...")));
                p2p.connect_to_peer(host, port).await;
            }
            Err(e) => println!("{}", util::error(&e)),
        }
    } else {
        println!("{}", util::error("Usage: connect <host> <port>"));
    }
}

fn cmd_peers(p2p: &Arc<P2P>) {
    let peers = p2p.get_peers();
    if peers.is_empty() {
        println!("{}", util::warning("No peers connected"));
        return;
    }
    let lines: Vec<String> = peers
        .iter()
        .enumerate()
        .map(|(i, addr)| format!("{CYAN}{}.{RESET} {GREEN}{addr}{RESET} [connected]", i + 1))
        .collect();
    println!(
        "\n{}",
        util::box_lines(&lines, &format!("Peers ({})", peers.len()), 50)
    );
}

async fn cmd_status(node: &NodeHandle, p2p: &Arc<P2P>) {
    let n = node.lock().await;
    let server = *p2p.server_port.lock().unwrap();
    let (syncing, _, _) = crate::p2p::sync::status(&n);
    let lines = vec![
        util::key_value(
            "Server",
            &server.map_or(format!("{RED}Offline{RESET}"), |p| {
                format!("{GREEN}:{p}{RESET}")
            }),
        ),
        util::key_value(
            "Peers",
            &format!("{YELLOW}{}{RESET} connected", p2p.get_peers().len()),
        ),
        util::key_value(
            "Syncing",
            &if syncing {
                format!("{YELLOW}Yes{RESET}")
            } else {
                format!("{GREEN}No{RESET}")
            },
        ),
        util::divider(35),
        util::key_value(
            "Blocks",
            &format!("{CYAN}{}{RESET}", n.blockchain.chain.len()),
        ),
        util::key_value(
            "Difficulty",
            &format!("{MAGENTA}{}{RESET}", n.blockchain.difficulty),
        ),
        util::key_value(
            "Mempool",
            &format!("{YELLOW}{}{RESET} tx", n.blockchain.mempool.len()),
        ),
        util::key_value(
            "Valid",
            &if n.blockchain.is_chain_valid() {
                format!("{GREEN}Yes{RESET}")
            } else {
                format!("{RED}No{RESET}")
            },
        ),
    ];
    println!("\n{}", util::box_lines(&lines, "Status", 50));
}

async fn cmd_sync(node: &NodeHandle, p2p: &Arc<P2P>) {
    let n = node.lock().await;
    let (syncing, _, _) = crate::p2p::sync::status(&n);
    drop(n);
    if syncing {
        println!("{}", util::warning("Already syncing..."));
    } else if p2p.trigger_sync().await {
        println!("{}", util::success("Sync request sent"));
    }
}

fn cmd_disconnect(p2p: &Arc<P2P>, args: &[&str]) {
    match args.first().and_then(|i| i.parse::<usize>().ok()) {
        Some(index) => match p2p.disconnect_peer(index) {
            Ok(_) => println!(
                "{}",
                util::success(&format!("Disconnected from peer {index}"))
            ),
            Err(e) => println!("{}", util::error(&e)),
        },
        None => {
            p2p.disconnect_all();
            println!("{}", util::success("Disconnected from all peers"));
        }
    }
}

// ---- Wallet commands ----

async fn cmd_wallet_create(node: &NodeHandle, args: &[&str]) {
    let name = args.first().copied().unwrap_or("");
    let mut n = node.lock().await;
    match n.wallets.create_wallet(name) {
        Ok(address) => {
            let lines = vec![
                util::key_value("Name", &format!("{CYAN}{name}{RESET}")),
                util::key_value(
                    "Address",
                    &format!("{YELLOW}{}{RESET}", util::shorten_address(&address)),
                ),
            ];
            println!("\n{}", util::box_lines(&lines, "💰 Wallet Created", 50));
            println!("{}\n", util::success("Wallet created successfully!"));
        }
        Err(e) => println!("{}", util::error(&format!("Error creating wallet: {e}"))),
    }
}

async fn cmd_wallets(node: &NodeHandle, args: &[&str]) {
    let n = node.lock().await;
    if args.first() == Some(&"all") {
        let entries: Vec<(String, i128)> = n
            .blockchain
            .balance_tracker
            .get_all_balances()
            .iter()
            .filter(|(_, b)| **b > 0)
            .map(|(a, b)| (a.clone(), *b))
            .collect();
        if entries.is_empty() {
            println!("{}", util::warning("No addresses with balance."));
            return;
        }
        let lines: Vec<String> = entries
            .iter()
            .enumerate()
            .map(|(i, (addr, bal))| {
                format!(
                    "{CYAN}{}.{RESET} {}\n   💎 {YELLOW}{}{RESET} coins",
                    i + 1,
                    util::shorten_address(addr),
                    util::fmt_micro_i(*bal)
                )
            })
            .collect();
        println!(
            "\n{}",
            util::box_lines(&lines, &format!("💰 All Addresses ({})", entries.len()), 50)
        );
    } else {
        let wallets = n.wallets.list_wallets();
        if wallets.is_empty() {
            println!(
                "{}",
                util::warning("No wallets found. Create one with: wallet-create <name>")
            );
            return;
        }
        let lines: Vec<String> = wallets
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let address = n.wallets.get_address(name).unwrap_or_default();
                let balance = n.blockchain.get_balance(&address);
                format!(
                    "{CYAN}{}.{RESET} {BRIGHT}{name}{RESET}\n   → {}\n   💎 {YELLOW}{}{RESET} coins",
                    i + 1,
                    util::shorten_address(&address),
                    util::fmt_micro_i(balance)
                )
            })
            .collect();
        println!(
            "\n{}",
            util::box_lines(
                &lines,
                &format!("💰 Managed Wallets ({})", wallets.len()),
                50
            )
        );
    }
}

async fn cmd_balance(node: &NodeHandle, args: &[&str]) {
    let name = args.first().copied().unwrap_or("");
    let n = node.lock().await;
    match n.wallets.get_address(name) {
        Ok(address) => {
            let balance = n.blockchain.get_balance(&address);
            let lines = vec![
                util::key_value("Wallet", &format!("{BRIGHT}{name}{RESET}")),
                util::key_value(
                    "Address",
                    &format!("{DIM}{}{RESET}", util::shorten_address(&address)),
                ),
                util::divider(35),
                util::key_value(
                    "Balance",
                    &format!("{GREEN}💎 {}{RESET} coins", util::fmt_micro_i(balance)),
                ),
            ];
            println!("\n{}", util::box_lines(&lines, "💰 Balance", 40));
        }
        Err(_) => println!(
            "{}",
            util::error(&format!(
                "Error checking balance: Wallet \"{name}\" not found"
            ))
        ),
    }
}

async fn cmd_address(node: &NodeHandle, name: &str) {
    let n = node.lock().await;
    match n.wallets.get_address(name) {
        Ok(address) => {
            println!("\n{CYAN}{name}{RESET} address:\n");
            println!("{YELLOW}{address}{RESET}\n");
            println!("{DIM}(Copy and share this address to receive coins){RESET}\n");
        }
        Err(_) => println!("{}", util::error(&format!("Wallet \"{name}\" not found"))),
    }
}

async fn cmd_history(node: &NodeHandle, name: &str) {
    let address = {
        let n = node.lock().await;
        match n.wallets.get_address(name) {
            Ok(a) => a,
            Err(e) => {
                println!("{}", util::error(&format!("Error: {e}")));
                return;
            }
        }
    };
    let history = {
        let n = node.lock().await;
        n.blockchain.get_transactions_history(&address)
    };

    if history.is_empty() {
        println!(
            "{}",
            util::info(&format!("No transactions found for {name}"))
        );
        return;
    }

    println!("\n{CYAN}💰 Transaction History: {name}{RESET}");
    println!("{}", util::divider(50));
    for (i, tx) in history.iter().enumerate() {
        let icon = match tx.history_type.as_str() {
            "MINING_REWARD" => "⛏",
            "SENT" => "📤",
            _ => "📥",
        };
        let (color, sign) = if tx.history_type == "SENT" {
            (RED, "-")
        } else {
            (GREEN, "+")
        };
        println!("  {DIM}{}.{RESET} {icon} {}", i + 1, tx.history_type);
        println!(
            "     {DIM}Date:{RESET} {}",
            util::fmt_timestamp(tx.timestamp)
        );
        if tx.history_type != "MINING_REWARD" {
            println!("     {DIM}From:{RESET} {}", util::shorten_address(&tx.from));
            println!("     {DIM}To:{RESET}   {}", util::shorten_address(&tx.to));
        }
        println!(
            "     {DIM}Amount:{RESET} {color}{sign}{}{RESET} coins",
            util::fmt_micro(tx.amount)
        );
        if tx.fee > 0 {
            println!(
                "     {DIM}Fee:{RESET} {YELLOW}{}{RESET}",
                util::fmt_micro(tx.fee)
            );
        }
        println!("     {DIM}Block:{RESET} #{}", tx.block_index);
        println!("{}", util::divider(50));
    }

    let received: u64 = history
        .iter()
        .filter(|t| t.history_type != "SENT")
        .map(|t| t.amount)
        .sum();
    let sent: u64 = history
        .iter()
        .filter(|t| t.history_type == "SENT")
        .map(|t| t.amount)
        .sum();
    let fees: u64 = history
        .iter()
        .filter(|t| t.history_type == "SENT")
        .map(|t| t.fee)
        .sum();
    println!("\n  {BRIGHT}Summary:{RESET}");
    println!(
        "  {GREEN}Total Received: +{}{RESET} coins",
        util::fmt_micro(received)
    );
    println!("  {RED}Total Sent: -{}{RESET} coins", util::fmt_micro(sent));
    println!(
        "  {YELLOW}Total Fees: -{}{RESET} coins\n",
        util::fmt_micro(fees)
    );
}

async fn cmd_export(node: &NodeHandle, name: &str) {
    let n = node.lock().await;
    match (n.wallets.get_private_key(name), n.wallets.get_address(name)) {
        (Ok(sk), Ok(address)) => {
            println!("\n{RED}⚠ WARNING: KEEP THIS PRIVATE KEY SECRET!{RESET}");
            println!("{DIM}Anyone with this key can steal your coins.{RESET}\n");
            println!("{CYAN}Wallet:{RESET} {name}");
            println!(
                "{CYAN}Address:{RESET} {}\n",
                util::shorten_address(&address)
            );
            println!("{YELLOW}Private Key (hex):{RESET}");
            println!("{DIM}{sk}{RESET}");
        }
        _ => println!("{}", util::error(&format!("Wallet \"{name}\" not found"))),
    }
}

async fn cmd_import(node: &NodeHandle, name: &str) {
    {
        let n = node.lock().await;
        if n.wallets.has_wallet(name) {
            println!(
                "{}",
                util::error(&format!("Wallet \"{name}\" already exists"))
            );
            return;
        }
    }
    print!("Paste private key (hex format): ");
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return;
    }
    let mut n = node.lock().await;
    match n.wallets.import_wallet(name, input.trim()) {
        Ok(address) => {
            println!("{}", util::success(&format!("Wallet \"{name}\" imported!")));
            println!("{DIM}Address: {}{RESET}\n", util::shorten_address(&address));
        }
        Err(e) => println!("{}", util::error(&format!("Import failed: {e}"))),
    }
}

async fn cmd_wallet_delete(node: &NodeHandle, name: &str) {
    {
        let n = node.lock().await;
        if !n.wallets.has_wallet(name) {
            println!("{}", util::error(&format!("Wallet \"{name}\" not found")));
            return;
        }
    }
    print!("⚠ Delete wallet \"{name}\"? This cannot be undone [y/N]: ");
    use std::io::Write as _;
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    if input.trim().eq_ignore_ascii_case("y") {
        let mut n = node.lock().await;
        match n.wallets.delete_wallet(name) {
            Ok(_) => println!("{}", util::success(&format!("Wallet \"{name}\" deleted"))),
            Err(e) => println!("{}", util::error(&format!("Delete failed: {e}"))),
        }
    } else {
        println!("{}", util::info("Cancelled"));
    }
}

// ---- Transaction / Mining commands ----

async fn cmd_send(node: &NodeHandle, p2p: &Arc<P2P>, args: &[&str]) {
    let (Some(from), Some(to), Some(amount_str)) = (args.first(), args.get(1), args.get(2)) else {
        println!("{}", util::error("Usage: send <from> <to> <amount> [fee]"));
        return;
    };

    // Người gửi phải là wallet local (cần private key)
    let (from_address, private_key, public_key) = {
        let n = node.lock().await;
        match (
            n.wallets.get_address(from),
            n.wallets.get_private_key(from),
            n.wallets.get_public_key_hex(from),
        ) {
            (Ok(a), Ok(sk), Ok(pk)) => (a, sk, pk),
            _ => {
                println!(
                    "{}",
                    util::error(&format!("Sender wallet \"{from}\" not found"))
                );
                return;
            }
        }
    };

    // Người nhận: wallet name / hex address / prefix
    let to_address = match resolve_address(node, to).await {
        Ok(a) => a,
        Err(e) => {
            println!("{}", util::error(&e));
            return;
        }
    };
    let to_display = util::shorten_address(&to_address);

    let amount = match util::coins_str_to_micro(amount_str) {
        Ok(a) => a,
        Err(e) => {
            println!("{}", util::error(&e));
            return;
        }
    };
    let fee = match args.get(3) {
        Some(fee_str) => match util::coins_str_to_micro(fee_str) {
            Ok(f) => f,
            Err(e) => {
                println!("{}", util::error(&e));
                return;
            }
        },
        None => 0,
    };

    let mut tx = Transaction::new(&from_address, &to_address, amount, fee);
    if let Err(e) = tx.sign(&private_key, &public_key) {
        println!("{}", util::error(&format!("Signing failed: {e}")));
        return;
    }

    {
        let mut n = node.lock().await;
        if let Err(e) = n.blockchain.add_transaction(&tx) {
            println!("{}", util::error(&e));
            return;
        }
    }

    p2p.broadcast(&messages::transaction(&tx));

    let lines = vec![
        util::key_value("From", &format!("{CYAN}{from}{RESET}")),
        util::key_value("To", &format!("{CYAN}{to_display}{RESET}")),
        util::divider(35),
        util::key_value(
            "Amount",
            &format!("{YELLOW}{}{RESET} coins", util::fmt_micro(amount)),
        ),
        util::key_value(
            "Fee",
            &format!("{DIM}{}{RESET} coins", util::fmt_micro(fee)),
        ),
        util::key_value(
            "Total",
            &format!("{RED}-{}{RESET} coins", util::fmt_micro(amount + fee)),
        ),
    ];
    println!(
        "\n{}",
        util::box_lines(&lines, "📤 Transaction Created", 40)
    );
    println!(
        "{}",
        util::success(&format!("Broadcasted to {} peer(s)", p2p.get_peers().len()))
    );
    println!("{}\n", util::info("Wait for it to be mined into a block."));
}

async fn cmd_mine(node: &NodeHandle, p2p: &Arc<P2P>, wallet: &str) {
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

async fn cmd_automine(node: &NodeHandle, p2p: &Arc<P2P>, args: &[&str]) {
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

async fn cmd_stop_automine(node: &NodeHandle) {
    let mut n = node.lock().await;
    if let Some(task) = n.auto_mine_task.take() {
        task.abort();
        n.auto_mine = None;
        println!("{}", util::success("Auto-mine stopped"));
    } else {
        println!("{}", util::info("Auto-mine is not running"));
    }
}

// ---- Blockchain commands ----

async fn cmd_blockchain(node: &NodeHandle) {
    let n = node.lock().await;
    println!(
        "\n{CYAN}⛓ Blockchain ({} blocks){RESET}\n",
        n.blockchain.chain.len()
    );
    for block in &n.blockchain.chain {
        println!("{block}");
    }
}

async fn cmd_block(node: &NodeHandle, query: &str) {
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

async fn cmd_latest(node: &NodeHandle) {
    let n = node.lock().await;
    println!("\n{}", n.blockchain.get_latest_block());
}

async fn cmd_validate(node: &NodeHandle) {
    let n = node.lock().await;
    if n.blockchain.is_chain_valid() {
        println!("{}", util::success("Blockchain is valid"));
    } else {
        println!("{}", util::error("Blockchain is invalid!"));
    }
}

async fn cmd_stats(node: &NodeHandle) {
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

async fn cmd_tx(node: &NodeHandle, query: &str) {
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

async fn cmd_mempool(node: &NodeHandle) {
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

async fn cmd_fee(node: &NodeHandle) {
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

async fn cmd_reset(node: &NodeHandle) {
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
