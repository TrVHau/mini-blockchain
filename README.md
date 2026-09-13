# mini-blockchain-rs

A simple blockchain for learning purposes — Rust port của [mini-blockchain](../mini-blockchain) (JavaScript).

## Tính năng

- Proof of Work (SHA-256, difficulty tự điều chỉnh mỗi 10 blocks)
- Giao dịch ECDSA secp256k1 (ký/verify, double-spend protection)
- Merkle tree + Merkle proof
- Block reward có halving (16 coins, giảm 50% mỗi 50 blocks)
- P2P qua WebSocket: handshake, partial/full sync, relay block/transaction
- REST API (axum, bind 127.0.0.1): query chain/block/tx/balance/mempool, tạo ví, gửi tx, Merkle proof
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

## REST API

Khởi động với flag `-r/--rest <port>` (bind `127.0.0.1`):

```bash
cargo run -- -n node1 -p 3000 -a -r 8080
```

| Endpoint | Mô tả |
|---|---|
| `GET /info`, `GET /stats` | trạng thái node / thống kê chain |
| `GET /chain` | toàn bộ chain |
| `GET /blocks/<index\|hash>` | block theo index hoặc hash |
| `GET /blocks/<index>/proof/<txid>` | Merkle proof + verify |
| `GET /tx/<txid>` | chi tiết transaction |
| `GET /mempool`, `GET /fee` | tx đang chờ / ước tính fee |
| `GET /balance/<name\|address\|prefix>` | số dư (resolve như CLI) |
| `GET /wallets`, `POST /wallets {"name"}` | danh sách ví / tạo ví (private key trả về 1 lần) |
| `POST /transactions` | `{"from": "alice", "to": "bob", "amount": "12.5", "fee": "0.1"}` — sign + broadcast |

```bash
curl -s localhost:8080/info
curl -s -X POST localhost:8080/wallets -d '{"name":"alice"}'
curl -s -X POST localhost:8080/transactions -d '{"from":"alice","to":"bob","amount":"5"}'
```

## Test

```bash
cargo test
```
