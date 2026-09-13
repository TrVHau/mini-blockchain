# Kế hoạch cải thiện mini-blockchain-rs

> Tạo ngày 2026-09-13. Tài liệu kế hoạch — chưa triển khai.

## Context

mini-blockchain-rs là bản port Rust hoàn chỉnh của mini-blockchain JS (~2000 dòng, 14 file).
Core đã chạy tốt: PoW, ECDSA, Merkle, P2P sync, CLI REPL, có unit test ở các module logic.
Tuy nhiên còn các lỗ hổng correctness (double-spend protection), vùng mù test (toàn bộ
p2p/ + sync/ ~570 dòng chưa có test), chưa có CI, và một nhóm hàm `#[allow(dead_code)]`
đang chờ REST API.

---

## Phần 1 — Sửa bug + hardening (ưu tiên cao nhất)

### 1.1. Lỗ hổng double-spend: `spent_txids` không cập nhật khi nhận block/chain từ peer

- `apply_mined_block` (src/blockchain/chain.rs:297) insert txid vào `spent_txids`,
  nhưng `receive_block` (src/blockchain/chain.rs:139) **không** — block nhận từ peer
  được push vào chain mà txid không được đánh dấu spent.
- `receive_chain` (src/blockchain/chain.rs:192) thay toàn bộ chain nhưng `spent_txids`
  giữ nguyên giá trị cũ — sau khi sync chain dài hơn từ peer, **không txid nào** của
  chain mới nằm trong `spent_txids`.
- Hệ quả: một tx đã confirm (qua block/chain từ mạng) bị replay sẽ pass
  `validate_tx_not_duplicate` (đã khỏi mempool, không có trong spent_txids); nếu người
  gửi còn dư số dư, tx replay được mine lần nữa → debit hai lần.
- **Sửa** (gốc rễ, một chỗ): thêm helper `track_spent(&mut self, block: &Block)` dùng
  chung cho cả `receive_block` và `apply_mined_block`; trong `receive_chain` rebuild
  `spent_txids` từ chain mới (duyệt 1 lượt, cùng chỗ với `update_balance`).
- Test: replay tx đã confirm qua đường `receive_block`/`receive_chain` phải bị từ chối.

### 1.2. Test flaky: `wallet_manager_roundtrip` đổi current_dir toàn process

- src/wallet.rs:206-229 dùng `std::env::set_current_dir` — `cargo test` chạy test song
  song trong cùng process; mọi test khác chạm đường dẫn tương đối sẽ race.
- **Sửa**: thêm tham số base-dir cho `Storage::new` / `WalletManager::new` (mặc định
  `"data"` khi gọi từ production — không đổi hành vi), test truyền temp dir. Phụ lợi:
  mở đường cho integration test hermetic ở Phần 2 (không ghi vào `data/` của repo).

### 1.3. Dùng lại `get_block` / `get_block_by_hash` trong CLI

- `cmd_block` (src/cli.rs:691) tự viết logic tìm theo index/hash prefix trong khi
  `BlockChain::get_block` / `get_block_by_hash` (src/blockchain/chain.rs:83-91) đang
  `#[allow(dead_code)]`. Refactor `cmd_block` gọi 2 hàm này rồi bỏ 2 annotation.

### 1.4. Ghi nhận (không sửa — đánh dấu `ponytail:`)

- `receive_chain` validate PoW theo `self.difficulty` cục bộ, không suy difficulty từ
  chain nhận được → chain của peer mine ở difficulty khác có thể bị từ chối sai.
- `handle_new_block` (src/p2p/mod.rs:310) đọc `latest_index` trong lock rồi dùng ngoài
  lock — race nhỏ, chấp nhận được cho learning project.
- `validate_timestamp` phụ thuộc đồng hồ cục bộ — node lệch giờ sẽ từ chối chain hợp lệ.

---

## Phần 2 — Test coverage

### 2.1. Unit tests bổ sung (chi phí thấp)

- `storage.rs`: round-trip save/load blockchain, file hỏng → `None` (cần base-dir từ 1.2).
- `validators.rs`: tamper từng trường (index, previous_hash, nonce, merkle root,
  coinbase amount, difficulty) → mỗi case phải fail đúng chỗ.
- `p2p/messages.rs`: `parse` với JSON thiếu `type`, data null, round-trip encode/decode.

### 2.2. Integration test P2P (file `tests/p2p_sync.rs`)

- Spawn 2 node với temp dir riêng (dựa trên base-dir từ 1.2) + 2 `P2P` server trên
  ephemeral port (`TcpListener::bind(("127.0.0.1", 0))`), connect, rồi assert:
  1. Mine block ở node A → node B nhận qua NEW_BLOCK relay, chain height khớp.
  2. Node B khởi động sau, chain ngắn hơn → REQUEST_CHAIN/RECEIVE_BLOCKS sync đủ height.
  3. `send` tx ở node A → xuất hiện trong mempool node B.
  4. (Regression của 1.1) Replay tx đã confirm từ node B → bị từ chối.

---

## Phần 3 — Tooling / CI

- `rustfmt.toml` tối giản (nếu code hiện tại đã theo default thì chỉ cần chạy
  `cargo fmt` một lần cho toàn repo).
- Chạy `cargo clippy --all-targets` và sửa warnings (trạng thái hiện tại chưa kiểm
  chứng được).
- `.github/workflows/ci.yml`: 3 job trên ubuntu-latest — `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`. Chỉ có ý nghĩa nếu repo
  push lên GitHub.

---

## Phần 4 — REST API server (dựa trên nhóm hàm dead-code sẵn có)

Thêm module `src/api.rs` dùng **axum** (thêm 1 dependency duy nhất; share sẵn
`NodeHandle = Arc<tokio::sync::Mutex<Node>>` nên wiring gần như miễn phí):

- `GET /info`, `GET /stats` — `Node::get_node_info`, `BlockChain::get_stats`
- `GET /chain`, `GET /blocks/:index_or_hash` — `get_block`, `get_block_by_hash` (1.3)
- `GET /tx/:txid`, `GET /mempool`, `GET /fee`
- `GET /balance/:address` (+ `?address=` prefix match như `resolve_address`)
- `GET /blocks/:index/merkle-proof/:txid` — `verify_transaction` / `merkle::get_proof`
- `POST /wallets` (tạo ví, trả private key một lần), `GET /wallets`
- `POST /transactions` — sign + `add_transaction` + broadcast (dùng lại luồng `cmd_send`)
- Flag CLI mới `-r/--rest <port>` trong `main.rs`; README cập nhật bảng endpoint.

Ràng buộc bảo mật cho learning project: API chỉ bind `127.0.0.1`, POST /transactions
yêu cầu wallet local; không expose private key qua GET.

---

## Thứ tự triển khai đề xuất

| # | Việc | Phụ thuộc |
|---|------|-----------|
| 1 | 1.1 spent_txids (bug double-spend) | — |
| 2 | 1.2 base-dir cho Storage/WalletManager | — |
| 3 | 1.3 reuse get_block trong CLI | — |
| 4 | 2.1 + 2.2 test | 1.2 (temp dir), 1.1 (regression test) |
| 5 | 3 fmt/clippy/CI | sau 4 để CI xanh ngay |
| 6 | 4 REST API | sau 1.3 (dùng lại get_block) |

## Verification

- Mỗi phần: `cargo test` xanh, `cargo clippy --all-targets -- -D warnings` sạch.
- Phần 1.1: test replay-tx-sau-receive-block/receive-chain fail trước, pass sau.
- Phần 2.2: `cargo test --test p2p_sync` chạy 2 node thật trên localhost.
- Phần 4: chạy `cargo run -- -n node1 -p 3000 -a -r 8080`, `curl` từng endpoint,
  kiểm tra mine qua CLI rồi query qua API thấy cùng block.
