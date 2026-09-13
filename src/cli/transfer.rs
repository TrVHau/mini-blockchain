//! CLI — lệnh transfer: send.

use std::sync::Arc;

use crate::blockchain::transaction::Transaction;
use crate::node::NodeHandle;
use crate::p2p::{messages, P2P};
use crate::util::{self, *};

use super::resolve_address;

pub(super) async fn cmd_send(node: &NodeHandle, p2p: &Arc<P2P>, args: &[&str]) {
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
        if let Err(e) = n.add_transaction(&tx) {
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
