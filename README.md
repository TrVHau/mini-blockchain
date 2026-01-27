# ⛓ Mini Blockchain

Một blockchain đơn giản để học tập với P2P networking, proof-of-work mining và digital signatures.

## Cài đặt

```bash
npm install
```

## Chạy

```bash
npm start                           # Chế độ mặc định
node src/index.js -n node1 -p 3000 -a   # Node với ID, port, auto-start
```

### Options

```
-n, --node <id>      Node ID (cho storage riêng)
-p, --port <port>    Port cho P2P server
-c, --connect <addr> Kết nối đến peer (host:port)
-a, --auto           Tự động start server
```

## Lệnh CLI

### Network

| Lệnh                    | Alias | Mô tả              |
| ----------------------- | ----- | ------------------ |
| `open <port>`           | `o`   | Mở P2P server      |
| `connect <host> <port>` | `c`   | Kết nối đến peer   |
| `peers`                 | `p`   | Danh sách peers    |
| `status`                | `s`   | Trạng thái node    |
| `sync`                  |       | Đồng bộ blockchain |

### Wallet

| Lệnh                   | Alias | Mô tả             |
| ---------------------- | ----- | ----------------- |
| `wallet-create <name>` | `wc`  | Tạo wallet mới    |
| `wallets`              | `wl`  | Danh sách wallets |
| `balance <name>`       | `bal` | Xem số dư         |

### Transaction

| Lệnh                              | Mô tả                    |
| --------------------------------- | ------------------------ |
| `send <from> <to> <amount> [fee]` | Gửi coins                |
| `mempool` (`mp`)                  | Xem pending transactions |

### Mining

| Lệnh            | Alias | Mô tả         |
| --------------- | ----- | ------------- |
| `mine <wallet>` | `m`   | Đào block mới |

### Blockchain

| Lệnh            | Alias | Mô tả             |
| --------------- | ----- | ----------------- |
| `blockchain`    | `bc`  | Xem toàn bộ chain |
| `block <index>` | `b`   | Xem block cụ thể  |
| `latest`        | `l`   | Block mới nhất    |
| `validate`      | `v`   | Kiểm tra chain    |

## Demo: 2 Nodes

**Terminal 1:**

```bash
node src/index.js -n alice -p 3000 -a
# Trong CLI:
wallet-create alice
mine alice
mine alice
```

**Terminal 2:**

```bash
node src/index.js -n bob -p 3001 -a -c localhost:3000
# Trong CLI:
blockchain  # Tự động sync 3 blocks
wallet-create bob
```

**Terminal 1:**

```bash
send alice bob 10
mine alice
```

**Terminal 2:**

```bash
blockchain  # Thấy transaction
balance bob # 10 coins
```

## Cấu trúc

```
src/
├── blockchain/     # Block, BlockChain, Transaction
├── cli/            # Vorpal CLI commands
├── p2p/            # P2P networking, sync
├── storage/        # Persistent storage
├── util/           # Helpers (UI, Validator)
├── wallet/         # Wallet management
└── config/         # Constants
```

## Tech Stack

- Node.js + WebSocket (ws)
- Vorpal CLI
- SHA-256 + ECDSA signatures
