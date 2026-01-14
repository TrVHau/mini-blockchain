# 🔗 Mini Blockchain

Blockchain peer-to-peer đơn giản để học tập và thử nghiệm. Mỗi terminal là một node trong mạng blockchain phân tán.

## ⚡ Cài đặt

```bash
npm install
npm start
```

## 🎯 Tính năng

### Network

- **P2P networking** - Kết nối giữa các nodes
- **Block synchronization** - Đồng bộ blockchain tự động
- **Transaction broadcasting** - Broadcast giao dịch đến toàn mạng

### Wallet & Transactions

- **Wallet management** - Tạo và quản lý ví
- **Digital signatures** - Ký giao dịch bằng private key
- **Transaction validation** - Kiểm tra chữ ký, balance, và pending transactions

### Mining

- **Proof of Work** - Đào block với độ khó tùy chỉnh (1-6)
- **Mining rewards** - Phần thưởng cho miner (50 coins + fees)
- **Mempool** - Transaction pool trước khi mine vào block

### Data

- **Import/Export** - Lưu và load blockchain từ file JSON
- **Transaction history** - Xem lịch sử giao dịch của wallet
- **Chain validation** - Kiểm tra tính toàn vẹn của blockchain

## 📚 Lệnh CLI

### Network Commands

```bash
open <port>                    # Mở server P2P
connect <host> <port>          # Kết nối đến peer
peers                          # Danh sách peers
status                         # Trạng thái node
close-server                   # Đóng server
disconnect <index>             # Ngắt kết nối peer
disconnect-all                 # Ngắt tất cả peers
```

### Wallet Commands

```bash
wallet-create <name>           # Tạo ví mới
wallets                        # List wallets
wallets all                    # List tất cả addresses có balance
balance <name>                 # Kiểm tra số dư
history <wallet>               # Xem lịch sử giao dịch
```

### Transaction Commands

```bash
send <from> <to> <amount> [fee]  # Gửi coins
mempool                          # Xem pending transactions
```

### Mining Commands

```bash
mine <wallet> [data]           # Đào block mới
mine-mempool <wallet>          # Đào tất cả tx trong mempool
difficulty <level>             # Đặt độ khó (1-6)
```

### Blockchain Commands

```bash
blockchain                     # Xem toàn bộ chain
block <index>                  # Chi tiết 1 block
latest                         # Block mới nhất
validate                       # Kiểm tra chain hợp lệ
```

### Utility Commands

```bash
export <filename>              # Export chain ra file
import <filename>              # Import chain từ file
clear                          # Xóa màn hình
help                          # Xem tất cả lệnh
```

## 🚀 Ví dụ: Chạy mạng 3 nodes

### Node 1 (Alice)

```bash
npm start
BLOCKCHAIN => open 3001
BLOCKCHAIN => wallet-create Alice
BLOCKCHAIN => mine Alice "Genesis node"
```

### Node 2 (Bob)

```bash
npm start
BLOCKCHAIN => open 3002
BLOCKCHAIN => connect localhost 3001
BLOCKCHAIN => wallet-create Bob
BLOCKCHAIN => mine Bob
BLOCKCHAIN => send Bob Alice 10 1
```

### Node 3 (Charlie)

```bash
npm start
BLOCKCHAIN => open 3003
BLOCKCHAIN => connect localhost 3001
BLOCKCHAIN => wallet-create Charlie
BLOCKCHAIN => mine-mempool Charlie
BLOCKCHAIN => history Charlie
```

## 📁 Cấu trúc dự án

```
src/
├── blockchain/          # Core blockchain logic
│   ├── Block.js
│   ├── BlockChain.js
│   ├── Transaction.js
│   └── CoinbaseTransaction.js
├── wallet/             # Wallet & balance tracking
│   ├── Wallet.js
│   ├── WalletManager.js
│   └── BalanceTracker.js
├── p2p/                # Peer-to-peer networking
│   ├── P2P.js
│   ├── Messages.js
│   └── message-type.js
├── cli/                # Command line interface
│   ├── cli.js
│   └── commands/       # Commands grouped by function
│       ├── network.js
│       ├── wallet.js
│       ├── transaction.js
│       ├── mining.js
│       ├── blockchain.js
│       ├── history.js
│       └── utility.js
└── util/               # Helper functions
    └── AddressHelper.js
```

## 🔧 Công nghệ

- **Node.js** - Runtime
- **ws** - WebSocket cho P2P
- **crypto** - SHA-256 hashing & signatures
- **vorpal** - Interactive CLI

## 🎓 Mục đích học tập

Dự án này giúp hiểu rõ:

- Cách blockchain lưu trữ và liên kết blocks
- Proof of Work mining algorithm
- Digital signatures (public/private key)
- P2P network synchronization
- Transaction validation và mempool
- Consensus trong distributed system
- UTXO tracking và balance calculation

## 📝 Notes

- **Mining reward**: 50 coins per block
- **Default difficulty**: 4 (có thể thay đổi 1-6)
- **Data storage**: In-memory (có thể export/import)
- **Network**: Local WebSocket (không internet)
- **Wallet format**: RSA public/private keys

---

Made for learning blockchain fundamentals 🎓
