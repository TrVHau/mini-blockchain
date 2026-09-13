//! CLI — lệnh wallet: wallet-create, wallets, balance, address, history,
//! export, import, wallet-delete.

use crate::node::NodeHandle;
use crate::util::{self, *};

pub(super) async fn cmd_wallet_create(node: &NodeHandle, args: &[&str]) {
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

pub(super) async fn cmd_wallets(node: &NodeHandle, args: &[&str]) {
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

pub(super) async fn cmd_balance(node: &NodeHandle, args: &[&str]) {
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

pub(super) async fn cmd_address(node: &NodeHandle, name: &str) {
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

pub(super) async fn cmd_history(node: &NodeHandle, name: &str) {
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

pub(super) async fn cmd_export(node: &NodeHandle, name: &str) {
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

pub(super) async fn cmd_import(node: &NodeHandle, name: &str) {
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

pub(super) async fn cmd_wallet_delete(node: &NodeHandle, name: &str) {
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
