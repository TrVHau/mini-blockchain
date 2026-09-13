# mini-blockchain

Một blockchain mini viết bằng Rust, để học cách blockchain hoạt động: đào block,
ký giao dịch, đồng bộ giữa các node qua P2P.

> Port Rust hoàn chỉnh của project mini-blockchain JavaScript cũ (đã ngừng phát triển).
> Số dư ví là dữ liệu local — reset chain là mất, không phải tiền thật.

## Bắt đầu trong 2 phút

```bash
cargo build --release
cargo run -- -n node1 -p 3000 -a
```

Bạn đang chạy 1 node. Mở terminal thứ hai, thêm 1 node nữa vào mạng:

```bash
cargo run -- -n node2 -p 3001 -a -c localhost:3000
```

Node 2 tự kết nối tới node 1 và đồng bộ chain. Thêm node 3, 4... tương tự
(`-c localhost:3000` là địa chỉ node bất kỳ đã chạy — mesh tự lo phần còn lại).

## Làm quen qua CLI

Gõ các lệnh sau trong REPL của node 1:

```
⛓ node1 ➜ wallet-create Alice      # tạo ví Alice
⛓ node1 ➜ mine Alice               # đào 1 block, Alice nhận 16 coins
⛓ node1 ➜ wallet-create Bob
⛓ node1 ➜ send Alice Bob 5          # chuyển 5 coins (tx vào mempool, chờ confirm)
⛓ node1 ➜ mine Alice               # đào block chứa tx trên
⛓ node1 ➜ balance Bob              # -> 5
⛓ node1 ➜ validate                 # -> chain hợp lệ
```

Gõ `help` để xem tất cả lệnh:

| Nhóm      | Lệnh                                                                                    |
| --------- | --------------------------------------------------------------------------------------- |
| Network   | `open <port>` · `connect <host> <port>` · `peers` · `status` · `sync` · `disconnect [idx]` · `close` |
| Wallet    | `wallet-create <name>` · `wallets [all]` · `balance <name>` · `address <name>` · `history <name>` · `export <name>` · `import <name>` · `wallet-delete <name>` |
| Mining    | `mine <wallet>` · `automine <wallet> [interval]` · `stopautomine`                        |
| Chuyển tiền | `send <from> <to> <amount> [fee]`                                                     |
| Chain     | `blockchain` · `block <idx\|hash>` · `latest` · `validate` · `stats` · `tx <txid>` · `mempool` · `fee` · `proof <idx> <txid>` · `reset` |

Lệnh nhận tên ví (`Alice`), địa chỉ đầy đủ (64 ký tự hex), hoặc tiền tố địa chỉ.

## Tính năng

- **Proof of Work** — SHA-256, difficulty ghi trong block header, tự điều chỉnh
  mỗi 10 blocks theo thời gian đào thực tế (nhanh quá thì tăng, chậm quá thì giảm).
- **Giao dịch có chữ ký** — ECDSA secp256k1. Tx chỉ hợp lệ khi `from` khớp địa chỉ
  suy ra từ public key đã ký, có txid, đúng loại TRANSFER. Double-spend bị chặn
  qua `spent_txids`.
- **Merkle tree** — root trong block header, chứng minh tx thuộc block bằng
  Merkle proof (`proof` trong CLI, `/blocks/<i>/proof/<txid>` trong API).
- **Block reward có halving** — 16 coins/block, giảm 50% mỗi 50 blocks.
- **P2P qua WebSocket** — handshake, sync từng phần hoặc cả chain, relay
  block/transaction cho các peer khác.
- **Peer discovery mesh động** — node mới join, cả mesh tự biết và tự connect
  (tối đa 50 peer). Không cần cấu hình thủ công.
- **REST API** (axum, chỉ bind 127.0.0.1) — xem chain, ví, mempool; tạo ví; gửi tx.
- **Lưu trữ theo node** — `data/nodes/<id>/`, cả chain lẫn mempool sống sót qua
  restart. Ghi atomic (không sợ file hỏng giữa chừng), wallet file chmod 600.
- **CLI REPL** — mọi thao tác ở trên đều làm được từ terminal.

## REST API

Khởi động với `-r <port>`:

```bash
cargo run -- -n node1 -p 3000 -a -r 8080
```

| Endpoint                                 | Mô tả                                          |
| ---------------------------------------- | ---------------------------------------------- |
| `GET /info`, `GET /stats`                | trạng thái node / thống kê chain               |
| `GET /chain`                             | toàn bộ chain                                  |
| `GET /blocks/<index\|hash>`              | block theo index hoặc hash                     |
| `GET /blocks/<index>/proof/<txid>`       | Merkle proof + verify                          |
| `GET /tx/<txid>`                         | chi tiết transaction                           |
| `GET /mempool`, `GET /fee`               | tx đang chờ / ước tính fee                     |
| `GET /balance/<name\|address\|prefix>`   | số dư (resolve tên như CLI)                    |
| `GET /wallets`, `POST /wallets`          | danh sách ví / tạo ví `{"name": "alice"}`      |
| `POST /transactions`                     | gửi tx (xem dưới)                              |

```bash
curl -s localhost:8080/info
curl -s -X POST localhost:8080/wallets -d '{"name":"alice"}'
curl -s -X POST localhost:8080/transactions \
     -d '{"from":"alice","to":"bob","amount":"12.5","fee":"0.1"}'
```

Tạo ví qua API trả về private key đúng 1 lần — lưu lại nếu cần.
`POST /transactions` yêu cầu `from` là ví local (cần private key để ký).

## Protocol P2P

WebSocket, JSON `{type, data}`. Các loại message:

- **Kết nối / discovery**: `HANDSHAKE`, `HANDSHAKE_ACK`, `PEERS`
- **Sync chain**: `REQUEST_CHAIN`, `RECEIVE_CHAIN`, `REQUEST_BLOCKS_FROM`, `RECEIVE_BLOCKS`, `REQUEST_LATEST`, `NEW_BLOCK`
- **Giao dịch**: `TRANSACTION`

`PEERS` là danh sách `host:listenPort` của các peer đang kết nối — node nhận tự
connect tới addr chưa biết. Broadcast ngay khi mesh thay đổi (cập nhật tức thì)
và định kỳ 60s (tự lành khi event bị miss).

## Thiết kế đáng chú ý

- **Tiền là số nguyên** — micro-coin (1 coin = 1.000.000 micro), không
  floating-point. CLI/API nhập/xuất dạng thập phân (`send Alice Bob 12.5`).
- **Keys hex** — private key 32 bytes, public key nén 33 bytes. Address =
  sha256(public key), 64 ký tự hex. Ai có private key chi tiêu được ví — giữ kín.
- **Difficulty là một hàm deterministic của chain** — miner và validator dùng
  chung một rule (`expected_difficulty`): ngoài kỳ retarget giữ nguyên, tại
  boundary ±1 theo thời gian đào. Hai bên không thể lệch nhau.
- **Không tin gì từ mạng** — block nhận từ peer được validate toàn bộ: PoW,
  linkage, Merkle root, coinbase, và **từng tx bên trong** (chữ ký, `from` khớp
  public key, double-spend, số dư cộng dồn). Mempool load từ disk cũng được
  re-validate.
- **Concurrent đúng cách** — state dùng chung qua `Arc<Mutex<Node>>`; PoW chạy
  ngoài lock (spawn_blocking) nên node vẫn phản hồi CLI/P2P trong lúc đào.
- **Danh tính peer** = `host:listenPort` từ handshake; connection song song do
  race 2 node connect nhau đồng thời bị phát hiện và bỏ bản dư.

## Test

```bash
cargo test          # 58 test: unit + P2P integration (relay, sync, discovery, mesh)
cargo clippy --all-targets -- -D warnings
cargo fmt --check   # CI (GitHub Actions) chạy đủ 3 lệnh này
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
├── storage.rs           # load/save JSON theo node (atomic write)
├── merkle.rs            # Merkle tree + proof
├── crypto.rs            # secp256k1 sign/verify, address derivation
├── util.rs              # UI helpers, validators, micro-coin parse/format
└── config.rs            # constants (difficulty, reward, limits)
```
