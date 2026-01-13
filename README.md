# Mini Blockchain

Ứng dụng blockchain đơn giản với giao diện dòng lệnh (CLI), hỗ trợ P2P networking, wallet management và mining.

## Tính năng

### Blockchain

- Tạo và quản lý chuỗi khối với Proof of Work (PoW)
- Genesis block tự động
- Xác thực tính toàn vẹn của chuỗi
- Coinbase transaction cho miner reward

### Wallet & Transaction

- Tạo và quản lý ví (ECDSA key pairs)
- Gửi và nhận giao dịch
- Transaction fee
- Theo dõi số dư tài khoản
- Ký và xác thực giao dịch

### Mining

- Mining với độ khó tùy chỉnh
- Block reward cho miner
- Mempool cho pending transactions
- Transaction fees tự động tính toán

### P2P Network

- Kết nối peer-to-peer qua WebSocket
- Đồng bộ blockchain giữa các node
- Broadcast blocks và transactions
- Xử lý multiple peers

## Cài đặt

```bash
npm install
```

## Sử dụng

### Khởi động ứng dụng

```bash
npm start
```

### Các lệnh chính

#### Network Commands

- `open <port>` - Mở server P2P trên port
- `connect <host> <port>` - Kết nối đến peer
- `peers` - Xem danh sách peers đã kết nối
- `status` - Xem trạng thái node

#### Wallet Commands

- `create-wallet <name>` - Tạo ví mới
- `wallets` - Liệt kê tất cả ví
- `balance <address>` - Kiểm tra số dư

#### Transaction Commands

- `send <from> <to> <amount> [fee]` - Gửi tiền
- `mempool` - Xem pending transactions

#### Mining Commands

- `mine <miner-address>` - Mine block mới
- `difficulty [level]` - Xem/thay đổi độ khó mining

#### Blockchain Commands

- `blockchain` - Xem toàn bộ chuỗi
- `block <index>` - Xem thông tin block cụ thể
- `latest` - Xem block mới nhất
- `validate` - Kiểm tra tính hợp lệ của blockchain

#### Utility Commands

- `export <filename>` - Xuất blockchain ra file
- `import <filename>` - Nhập blockchain từ file
- `clear` - Xóa màn hình

## Cấu trúc thư mục

```
src/
├── blockchain/          # Core blockchain logic
│   ├── Block.js
│   ├── BlockChain.js
│   ├── Transaction.js
│   └── CoinbaseTransaction.js
├── wallet/              # Wallet management
│   ├── Wallet.js
│   ├── WalletManager.js
│   └── BalanceTracker.js
├── p2p/                 # P2P networking
│   ├── P2P.js
│   ├── Messages.js
│   └── message-type.js
├── cli/                 # Command line interface
│   └── cli.js
└── util/                # Helper utilities
    └── AddressHelper.js
```

## Dependencies

- `vorpal` - Interactive CLI framework
- `ws` - WebSocket client/server
- `readline-sync` - Synchronous input handling

## Tác giả

TrVHau

## License

ISC
