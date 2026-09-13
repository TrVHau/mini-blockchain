# mini-blockchain-rs

A simple blockchain for learning purposes — Rust port của [mini-blockchain](../mini-blockchain) (JavaScript).

## Tính năng

- Proof of Work (SHA-256, difficulty tự điều chỉnh mỗi 10 blocks)
- Giao dịch ECDSA secp256k1 (ký/verify, double-spend protection)
- Merkle tree + Merkle proof
- Block reward có halving (16 coins, giảm 50% mỗi 50 blocks)
- P2P qua WebSocket: handshake, partial/full sync, relay block/transaction
- Lưu trữ JSON theo node (`data/nodes/<id>/`)
- CLI REPL với đầy đủ lệnh (gõ `help`)

## Khác biệt so với bản JS

- **u64 micro-coins**: 1 coin = 1_000_000 micro — không lỗi floating-point. CLI vẫn nhập/xuất dạng thập phân (`send Alice Bob 12.5`).
- **Keys hex** thay PEM: private key 32 bytes hex, public key compressed 33 bytes hex (`import` nhận hex private key).
- **Merkle proof hoạt động đúng**: bản JS tính proof không tính coinbase leaf nên proof không khớp root khi block có coinbase; bản Rust dùng chung leaf list với root.

## Chạy

```bash
cargo build --release

# Node 1 (terminal 1)
cargo run -- -n node1 -p 3000 -a

# Node 2 (terminal 2) — tự connect tới node 1
cargo run -- -n node2 -p 3001 -a -c localhost:3000

# Node 3
cargo run -- -n node3 -p 3002 -a -c localhost:3000
```

## Lệnh CLI

```
Network:    open <port> | connect <host> <port> | peers | status | sync | close | disconnect [idx]
Wallet:     wallet-create <name> | wallets [all] | balance <name> | address <name> | history <name>
            export <name> | import <name> | wallet-delete <name>
Mining:     mine <wallet> | automine <wallet> [interval] | stopautomine
Transfers:  send <from> <to> <amount> [fee]
Chain:      blockchain | block <idx|hash> | latest | validate | stats | tx <txid> | mempool | fee | reset
Misc:       help | exit
```

Thử nhanh:

```
wallet-create Alice     # tạo ví
mine Alice              # đào block, nhận 16 coins
wallet-create Bob
send Alice Bob 5        # chuyển 5 coins
mine Alice              # đưa tx vào block
balance Bob             # 5 coins
validate                # chain hợp lệ
```

## Protocol P2P

WebSocket, JSON `{type, data}` — giữ nguyên tên message của bản JS:
`HANDSHAKE`, `HANDSHAKE_ACK`, `REQUEST_CHAIN`, `RECEIVE_CHAIN`, `REQUEST_LATEST`,
`REQUEST_BLOCKS_FROM`, `RECEIVE_BLOCKS`, `NEW_BLOCK`, `TRANSACTION`.

Lưu ý: không interop với node JS (khác định dạng key/amount) — chạy mạng toàn Rust.

## Test

```bash
cargo test
```
