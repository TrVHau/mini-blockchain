# ⛓ Mini Blockchain

Một blockchain đầy đủ tính năng để học tập với:

- **P2P Networking** - Kết nối nhiều nodes
- **Proof-of-Work Mining** - Đào block với difficulty adjustment
- **Digital Signatures** - Xác thực giao dịch bằng ECDSA
- **Merkle Tree** - Verify transactions hiệu quả
- **Double Spend Protection** - Chống chi tiêu kép
- **Halving** - Giảm phần thưởng theo thời gian

## 🚀 Cài đặt

```bash
npm install
```

## 🎮 Chạy

```bash
# Chế độ tương tác
npm start

# Node với cấu hình cụ thể
node src/index.js -n node1 -p 3000 -a

# Xem help
node src/index.js -h
```

### Options

| Option                 | Mô tả                               |
| ---------------------- | ----------------------------------- |
| `-n, --node <id>`      | Node ID (mỗi node có storage riêng) |
| `-p, --port <port>`    | Port cho P2P server                 |
| `-c, --connect <addr>` | Kết nối đến peer (host:port)        |
| `-a, --auto`           | Tự động start server                |
| `-h, --help`           | Hiển thị help                       |

## 📖 Lệnh CLI

### 🌐 Network

| Lệnh                    | Alias | Mô tả                        |
| ----------------------- | ----- | ---------------------------- |
| `open <port>`           | `o`   | Mở P2P server                |
| `connect <host> <port>` | `c`   | Kết nối đến peer             |
| `peers`                 | `p`   | Danh sách peers đang kết nối |
| `status`                | `s`   | Trạng thái node              |
| `sync`                  |       | Đồng bộ blockchain từ peers  |

### 💰 Wallet

| Lệnh                   | Alias  | Mô tả                             |
| ---------------------- | ------ | --------------------------------- |
| `wallet-create <name>` | `wc`   | Tạo wallet mới                    |
| `wallets`              | `wl`   | Danh sách wallets của bạn         |
| `wallets all`          |        | Xem tất cả địa chỉ có số dư       |
| `balance <name>`       | `bal`  | Xem số dư wallet                  |
| `address <name>`       | `addr` | Hiển thị địa chỉ đầy đủ (để copy) |
| `history <name>`       | `h`    | Xem lịch sử giao dịch             |
| `export <name>`        |        | Export private key (backup)       |
| `import <name>`        |        | Import wallet từ private key      |

### 📤 Transaction

| Lệnh                              | Mô tả                                                              |
| --------------------------------- | ------------------------------------------------------------------ |
| `send <from> <to> <amount> [fee]` | Gửi coins. `<to>` có thể là tên wallet, địa chỉ đầy đủ hoặc prefix |

### ⛏ Mining

| Lệnh                      | Alias | Mô tả                                              |
| ------------------------- | ----- | -------------------------------------------------- |
| `mine <wallet>`           | `m`   | Đào block mới. Có thể dùng tên wallet hoặc địa chỉ |
| `automine <wallet> [sec]` | `am`  | Tự động đào khi có tx (mặc định 10s)               |
| `stopautomine`            | `sam` | Dừng auto-mine                                     |

### ⛓ Blockchain

| Lệnh            | Alias | Mô tả                           |
| --------------- | ----- | ------------------------------- |
| `blockchain`    | `bc`  | Xem toàn bộ chain               |
| `block <query>` | `b`   | Xem block theo index hoặc hash  |
| `latest`        | `l`   | Block mới nhất                  |
| `validate`      | `v`   | Kiểm tra chain hợp lệ           |
| `stats`         |       | Thống kê blockchain             |
| `tx <txid>`     |       | Xem transaction (hỗ trợ prefix) |
| `mempool`       | `mp`  | Xem pending transactions        |
| `fee`           |       | Ước tính phí giao dịch          |

## 🎯 Demo: 2 Nodes

### Terminal 1 (Alice)

```bash
node src/index.js -n alice -p 3000 -a

# Trong CLI:
wallet-create alice
mine alice
mine alice
address alice    # Copy địa chỉ này
```

### Terminal 2 (Bob)

```bash
node src/index.js -n bob -p 3001 -a -c localhost:3000

# Trong CLI:
blockchain       # Tự động sync từ Alice
wallet-create bob
address bob      # Copy địa chỉ này
```

### Terminal 1 (Alice gửi coins cho Bob)

```bash
# Paste địa chỉ Bob vào đây (hoặc dùng prefix)
send alice <bob-address> 10
mine alice       # Mine để confirm transaction
```

### Terminal 2 (Bob kiểm tra)

```bash
blockchain       # Thấy transaction
balance bob      # Kiểm tra số dư
```

## ⚙️ Cấu hình

Các constants có thể điều chỉnh trong `src/config/constants.js`:

| Constant                         | Mặc định | Mô tả                              |
| -------------------------------- | -------- | ---------------------------------- |
| `DEFAULT_DIFFICULTY`             | 4        | Độ khó mining ban đầu              |
| `TARGET_BLOCK_TIME`              | 30000ms  | Thời gian mục tiêu mỗi block       |
| `DIFFICULTY_ADJUSTMENT_INTERVAL` | 10       | Điều chỉnh difficulty mỗi N blocks |
| `INITIAL_MINING_REWARD`          | 16       | Phần thưởng block ban đầu          |
| `HALVING_INTERVAL`               | 50       | Halving mỗi N blocks               |
| `MAX_TRANSACTIONS_PER_BLOCK`     | 100      | Giới hạn TX mỗi block              |
| `CONFIRMATIONS_REQUIRED`         | 6        | Số confirmations để coi là final   |

## 🏗 Cấu trúc Project

```
src/
├── index.js              # Entry point
├── config/
│   └── constants.js      # Blockchain configuration
├── blockchain/
│   ├── Block.js          # Block với PoW
│   ├── BlockChain.js     # Core logic
│   ├── Transaction.js    # Signed transactions
│   └── CoinbaseTransaction.js
├── wallet/
│   ├── Wallet.js         # ECDSA key pair
│   ├── WalletManager.js  # Quản lý wallets
│   └── BalanceTracker.js # Theo dõi số dư
├── p2p/
│   ├── P2P.js           # WebSocket P2P
│   ├── Messages.js       # Protocol messages
│   ├── SyncManager.js    # Chain sync
│   └── ...
├── cli/
│   ├── cli.js           # Vorpal CLI
│   └── commands/        # CLI commands
├── storage/
│   └── Storage.js       # Persistent storage
└── util/
    ├── MerkleTree.js    # Merkle tree
    ├── AddressHelper.js # Address utilities
    └── UI.js            # CLI formatting
```

## 🔐 Tính năng bảo mật

- **ECDSA Signatures**: Mỗi transaction được ký bằng private key
- **Merkle Tree**: Verify transaction trong block O(log n)
- **Double Spend Protection**: Tracking txid đã sử dụng
- **Chain Validation**: Verify hash links và difficulty

## 📝 License

MIT
