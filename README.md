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

### 8. `status` hoặc `s`

Xem trạng thái tổng quan của node (port, số peer, số block, v.v.)

```
blockchain → status
```

### 9. `validate` hoặc `v`

Kiểm tra tính hợp lệ của toàn bộ blockchain

```
blockchain → validate
```

### 10. `block <index>` hoặc `b <index>`

Xem chi tiết một block cụ thể theo index

```
blockchain → block 0
blockchain → block 5
```

### 11. `latest` hoặc `l`

Xem block mới nhất trong chain

```
blockchain → latest
```

### 12. `difficulty <level>`

Thay đổi độ khó đào (1-6). Độ khó càng cao, đào càng lâu

```
blockchain → difficulty 3
blockchain → difficulty 5
```

### 13. `mempool`

Xem các transaction đang chờ trong mempool

```
blockchain → mempool
```

### 14. `add-tx <from> <to> <amount>`

Thêm transaction vào mempool

```
blockchain → add-tx Alice Bob 10
blockchain → add-tx Charlie Dave 25.5
```

### 15. `mine-mempool` hoặc `mm`

Đào tất cả các transaction trong mempool thành một block

```
blockchain → mine-mempool
```

### 16. `export <filename>`

Export blockchain ra file JSON

```
blockchain → export mychain.json
```

### 17. `import <filename>`

Import blockchain từ file JSON

```
blockchain → import mychain.json
```

### 18. `clear`

Xóa màn hình terminal

```
blockchain → clear
```

## Ví Dụ Luồng Hoạt Động

### Scenario 1: Khởi tạo mạng cơ bản

- Terminal 1: `open 3001`
- Terminal 2: `open 3002` → `connect localhost 3001`
- Terminal 3: `open 3003` → `connect localhost 3002`

### Scenario 2: Đào blocks đơn giản

Ở Terminal 1:

```
blockchain → mine "First Block"
blockchain → mine "Second Block"
```

Block này sẽ tự động được broadcast đến tất cả các peer đã kết nối.

### Scenario 3: Sử dụng Mempool & Transactions

```
blockchain → add-tx Alice Bob 50
blockchain → add-tx Bob Charlie 30
blockchain → add-tx Charlie Alice 20
blockchain → mempool
blockchain → mine-mempool
```

### Scenario 4: Kiểm tra trạng thái

```
blockchain → status
blockchain → latest
blockchain → validate
blockchain → block 0
```

### Scenario 5: Thay đổi difficulty

```
blockchain → difficulty 4
blockchain → mine "Test high difficulty"
```

(Bạn sẽ thấy quá trình đào lâu hơn!)

### Scenario 6: Export/Import blockchain

Export từ một node:

```
blockchain → export backup.json
```

Import vào node khác:

```
blockchain → import backup.json
blockchain → validate
```

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
