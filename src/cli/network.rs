//! CLI — lệnh network: open, connect, peers, status, sync, disconnect.

use std::sync::Arc;

use crate::node::NodeHandle;
use crate::p2p::P2P;
use crate::util::{self, *};

pub(super) async fn cmd_open(p2p: &Arc<P2P>, args: &[&str]) {
    match args.first().map(|p| util::validate_port(p)) {
        Some(Ok(port)) => p2p.start_server(port).await,
        _ => println!("{}", util::error("Usage: open <port> (1-65535)")),
    }
}

pub(super) async fn cmd_connect(p2p: &Arc<P2P>, args: &[&str]) {
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

pub(super) fn cmd_peers(p2p: &Arc<P2P>) {
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

pub(super) async fn cmd_status(node: &NodeHandle, p2p: &Arc<P2P>) {
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

pub(super) async fn cmd_sync(node: &NodeHandle, p2p: &Arc<P2P>) {
    let n = node.lock().await;
    let (syncing, _, _) = crate::p2p::sync::status(&n);
    drop(n);
    if syncing {
        println!("{}", util::warning("Already syncing..."));
    } else if p2p.trigger_sync().await {
        println!("{}", util::success("Sync request sent"));
    }
}

pub(super) fn cmd_disconnect(p2p: &Arc<P2P>, args: &[&str]) {
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
