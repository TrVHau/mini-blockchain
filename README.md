# mini-blockchain
# Mini Blockchain - Hướng Dẫn Sử Dụng

## Cài Đặt

```bash
npm install
```

## Chạy CLI

Mỗi terminal sẽ là một peer trong mạng P2P.

### Terminal 1 (Peer 1)

```bash
node src/index.js
```

Sau khi vào CLI, mở port để nhận kết nối:

```
blockchain → open 3001
```

### Terminal 2 (Peer 2)

Mở terminal mới:

```bash
node src/index.js
```

Mở port và kết nối đến Peer 1:

```
blockchain → open 3002
blockchain → connect localhost 3001
```

### Terminal 3 (Peer 3)

Mở terminal mới:

```bash
node src/index.js
```

Kết nối đến một trong các peer đã có:

```
blockchain → open 3003
blockchain → connect localhost 3001
```

## Các Lệnh CLI

### 1. `help`

Hiển thị danh sách các lệnh có sẵn

### 2. `open <port>` hoặc `o <port>`

Mở port để lắng nghe kết nối từ các peer khác

```
blockchain → open 3001
```

### 3. `connect <host> <port>` hoặc `c <host> <port>`

Kết nối đến một peer khác

```
blockchain → connect localhost 3001
```

### 4. `peers` hoặc `p`

Xem danh sách các peer đã kết nối

```
blockchain → peers
```

### 5. `blockchain` hoặc `bc`

Xem trạng thái blockchain hiện tại

```
blockchain → blockchain
```

### 6. `mine <data>` hoặc `m <data>`

Đào một block mới với dữ liệu

```
blockchain → mine "Hello World"
```

### 7. `discover` hoặc `d`

Khám phá các peer mới từ các peer đã kết nối

```
blockchain → discover
```

## Ví Dụ Luồng Hoạt Động

### Bước 1: Khởi tạo mạng

- Terminal 1: `open 3001`
- Terminal 2: `open 3002` → `connect localhost 3001`
- Terminal 3: `open 3003` → `connect localhost 3002`

### Bước 2: Kiểm tra kết nối

Ở mỗi terminal, chạy:

```
blockchain → peers
```

### Bước 3: Đào block

Ở Terminal 1:

```
blockchain → mine "First Block"
```

Block này sẽ tự động được broadcast đến tất cả các peer đã kết nối.

### Bước 4: Xem blockchain

Ở Terminal 2 hoặc 3:

```
blockchain → blockchain
```

Bạn sẽ thấy block mới được thêm vào chain!

## Cấu Trúc Dự Án

```
src/
  ├── index.js              # Entry point
  ├── blockchain/
  │   ├── Block.js          # Class Block
  │   ├── BlockChain.js     # Class BlockChain
  │   └── Transaction.js    # Class Transaction
  ├── cli/
  │   └── cli.js            # CLI interface với Vorpal
  ├── p2p/
  │   ├── P2P.js            # P2P network logic
  │   ├── Messages.js       # Message helper
  │   └── message-type.js   # Message types
  └── util/
      └── Wallet.js         # Wallet utilities
```

## Ghi Chú

- Mỗi peer có blockchain độc lập
- Khi một peer đào block mới, block đó sẽ được broadcast đến tất cả peer đã kết nối
- Blockchain sử dụng Proof of Work với difficulty = 2 (mặc định)
- Sử dụng WebSocket để giao tiếp giữa các peer
