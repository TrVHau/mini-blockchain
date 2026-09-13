//! Mini Blockchain (Rust) — port từ mini-blockchain JS.
//! Usage: mini-blockchain-rs [-n <node_id>] [-p <port>] [-c <host:port>] [-a] [-h]

mod api;
mod blockchain;
mod cli;
mod config;
mod crypto;
mod merkle;
mod node;
mod p2p;
mod storage;
mod util;
mod wallet;

use std::sync::Arc;

use node::{Node, NodeHandle};
use p2p::P2P;

const HELP: &str = r#"
Mini Blockchain - A simple blockchain for learning (Rust port)

Usage: mini-blockchain-rs [options]

Options:
  -p, --port <port>      Port to run the P2P server on
  -n, --node <id>        Node ID for persistent storage
  -c, --connect <addr>   Connect to peer (format: host:port)
  -a, --auto             Auto-start server on the specified port
  -r, --rest <port>      Start REST API server on 127.0.0.1:<port>
  -h, --help             Show this help message

Examples:
  # Start node 1 on port 3000
  mini-blockchain-rs -n node1 -p 3000 -a

  # Start node 2 on port 3001 and connect to node 1
  mini-blockchain-rs -n node2 -p 3001 -a -c localhost:3000

  # Start node with REST API on port 8080
  mini-blockchain-rs -n node1 -p 3000 -a -r 8080
"#;

fn main() {
    // Parse command line arguments
    let mut options = cli::Options {
        port: None,
        node_id: "default".to_string(),
        connect: None,
        auto_start: false,
        rest_port: None,
    };

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Lấy giá trị đứng sau flag, báo lỗi + thoát nếu thiếu
    fn value_of(args: &[String], flag: &str, i: &mut usize) -> String {
        *i += 1;
        match args.get(*i) {
            Some(v) => v.clone(),
            None => {
                eprintln!("Missing value for {flag}. Use -h for help.");
                std::process::exit(1);
            }
        }
    }

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-p" | "--port" => {
                let v = value_of(&args, arg, &mut i);
                match v.parse::<u16>() {
                    Ok(p) => options.port = Some(p),
                    Err(_) => {
                        eprintln!("Invalid port: {v}. Use -h for help.");
                        std::process::exit(1);
                    }
                }
            }
            "-n" | "--node" | "--id" => {
                options.node_id = value_of(&args, arg, &mut i);
            }
            "-c" | "--connect" => {
                options.connect = Some(value_of(&args, arg, &mut i)); // format: host:port
            }
            "-a" | "--auto" => options.auto_start = true,
            "-r" | "--rest" => {
                let v = value_of(&args, arg, &mut i);
                match v.parse::<u16>() {
                    Ok(p) => options.rest_port = Some(p),
                    Err(_) => {
                        eprintln!("Invalid REST port: {v}. Use -h for help.");
                        std::process::exit(1);
                    }
                }
            }
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown option: {other}. Use -h for help.");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Runtime + shared state
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    runtime.block_on(async move {
        let node: NodeHandle = Arc::new(tokio::sync::Mutex::new(Node::new(&options.node_id)));
        let p2p = P2P::new(node.clone());
        if let Some(port) = options.rest_port {
            tokio::spawn(api::serve(node.clone(), p2p.clone(), port));
        }
        cli::run(node, p2p, &options).await;
    });
}
