# mini-blockchain

A simple blockchain for learning purposes, viết bằng Rust.

> Bản Rust hoàn chỉnh của project mini-blockchain JavaScript cũ (đã ngừng phát triển).
> Số dư của ví là dữ liệu local — reset chain làm mất số dư (không phải tiền thật).

## Tính năng

- Proof of Work (SHA-256, difficulty nằm trong block header, tự điều chỉnh mỗi 10 blocks)
- Giao dịch ECDSA secp256k1 (ký/verify, double-spend protection qua `spent_txids`)
- Merkle tree + Merkle proof (CLI `proof` + REST API)
- Block reward có halving (16 coins, giảm 50% mỗi 50 blocks)
- P2P qua WebSocket: handshake, partial/full sync, relay block/transaction
- Peer discovery mesh động: node mới join/mesh thay đổi → tự cập nhật không cần
  reconnect (event-driven qua `PEERS` broadcast + tick 60s tự lành, full-mesh
  tối đa 50 peer)
- REST API (axum, bind 127.0.0.1): query chain/block/tx/balance/mempool, tạo ví, gửi tx, Merkle proof
- Lưu trữ JSON theo node (`data/nodes/<id>/`) — cả chain lẫn mempool, sống sót qua restart
- CLI REPL với đầy đủ lệnh (gõ `help`)

## Chạy

```bash
cargo build --release

# Node 1 (terminal 1)
cargo run -- -n node1 -p 3000 -a

# Node 2 (terminal 2) — tự connect tới node 1
cargo run -- -n node2 -p 3001 -a -c localhost:3000

# Node 3
cargo run -- -n node3 -p 3002 -a -c localhost:3000

# Node 4 — join sau: node 2, 3 tự connect lại trong ~60s (PEERS broadcast)
cargo run -- -n node4 -p 3003 -a -c localhost:3000
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

WebSocket, JSON `{type, data}`:
`HANDSHAKE`, `HANDSHAKE_ACK`, `PEERS`, `REQUEST_CHAIN`, `RECEIVE_CHAIN`,
`REQUEST_LATEST`, `REQUEST_BLOCKS_FROM`, `RECEIVE_BLOCKS`, `NEW_BLOCK`,
`TRANSACTION`.

`PEERS` = danh sách peer canonical (`host:listenPort`) của node gửi — node nhận
tự connect tới addr chưa biết. Broadcast ngay khi có connection mới (mesh cập
nhận tức thì) và định kỳ 60s (tự lành khi event bị miss).

## REST API

Khởi động với flag `-r/--rest <port>` (bind `127.0.0.1`):

```bash
cargo run -- -n node1 -p 3000 -a -r 8080
```

| Endpoint                                 | Mô tả                                                                               |
| ---------------------------------------- | ----------------------------------------------------------------------------------- |
| `GET /info`, `GET /stats`                | trạng thái node / thống kê chain                                                    |
| `GET /chain`                             | toàn bộ chain                                                                       |
| `GET /blocks/<index\|hash>`              | block theo index hoặc hash                                                           |
| `GET /blocks/<index>/proof/<txid>`       | Merkle proof + verify                                                               |
| `GET /tx/<txid>`                         | chi tiết transaction                                                                 |
| `GET /mempool`, `GET /fee`               | tx đang chờ / ước tính fee                                                           |
| `GET /balance/<name\|address\|prefix>`   | số dư (resolve như CLI)                                                              |
| `GET /wallets`, `POST /wallets {"name"}` | danh sách ví / tạo ví (private key trả về 1 lần)                                     |
| `POST /transactions`                     | `{"from": "alice", "to": "bob", "amount": "12.5", "fee": "0.1"}` — sign + broadcast |

```bash
curl -s localhost:8080/info
curl -s -X POST localhost:8080/wallets -d '{"name":"alice"}'
curl -s -X POST localhost:8080/transactions -d '{"from":"alice","to":"bob","amount":"5"}'
```

## Cấu trúc source

```
src/
├── main.rs              # parse args, runtime, start REST API
├── cli/                 # REPL: dispatch (mod.rs) + handlers theo nhóm
│   ├── network.rs       #   open/connect/peers/status/sync/disconnect
│   ├── wallets.rs       #   wallet-create/balance/history/export/import...
│   ├── mining.rs        #   mine/automine
│   ├── transfer.rs      #   send
│   └── chain_view.rs    #   blockchain/block/tx/mempool/stats...
├── api.rs               # REST API (axum)
├── node.rs              # shared state: chain + wallets + storage + sync
├── blockchain/
│   ├── block.rs         # Block + PoW
│   ├── transaction.rs   # Transaction/Coinbase (ký ECDSA, txid)
│   ├── chain/           # BlockChain: logic (mod.rs) + tests (tests.rs)
│   └── validators.rs    # validate tx/block/chain (stateless)
├── p2p/
│   ├── mod.rs           # P2P server/peers
│   ├── handler.rs       # connection task + message dispatch
│   ├── sync.rs          # SyncManager + watchdog
│   ├── messages.rs      # wire format
│   └── tests.rs         # integration test 2 node thật
├── wallet.rs            # WalletManager + BalanceTracker
├── storage.rs           # load/save JSON theo node
├── merkle.rs            # Merkle tree + proof
├── crypto.rs            # secp256k1 sign/verify, address derivation
├── util.rs              # UI helpers, validators, micro-coin parse/format
└── config.rs            # constants (difficulty, reward, limits)
```

## Kiến trúc

- **Đơn vị tiền**: u64 micro-coin (1 coin = 1_000_000 micro) — không lỗi floating-point.
  CLI/API nhập/xuất dạng thập phân (`send Alice Bob 12.5`).
- **Keys hex**: private key 32 bytes hex, public key compressed 33 bytes hex
  (`import` nhận hex private key; giữ bí mật — ai có key chi tiêu được ví).
- **Address** = sha256(public key) hex 64 ký tự.
- **Shared state**: `Arc<tokio::sync::Mutex<Node>>` dùng chung bởi CLI, P2P tasks và
  REST API. PoW chạy ngoài lock (spawn_blocking) để node vẫn phản hồi trong lúc mine.
- **Difficulty** nằm trong block header — node nhận validate theo difficulty khai báo
  (clamp `[MIN, MAX]`, không được giảm so với block trước), không đoán từ hash.
- **Trust boundary**: block nhận từ mạng được validate từng tx (chữ ký, double-spend,
  số dư cộng dồn) — không chỉ header.
- **Peer identity** = `host:listenPort` quảng bá qua handshake (PeerMap rekey từ
  ephemeral addr) — discovery connect lại được. Connection song song (race 2
  node connect nhau đồng thời) bị phát hiện và drop bản dư, mỗi cặp giữ đúng
  1 connection.
- **Double-spend**: mọi đường thêm block vào chain (mine / receive_block /
  receive_chain) đều đánh dấu txid vào `spent_txids`.

## Test

```bash
cargo test          # 50 test: unit (chain/validators/merkle/crypto/wallet/storage) + P2P integration (relay, sync, discovery, mesh động)
cargo clippy --all-targets -- -D warnings
cargo fmt --check   # CI chạy 3 lệnh này (GitHub Actions)
```
