//! Tests cho BlockChain (tách từ chain.rs cho file logic gọn lại).

use super::*;
use crate::crypto;

fn miner() -> (String, String, String) {
    let (sk, pk) = crypto::generate_keypair();
    let addr = crypto::address_from_public_hex(&pk).unwrap();
    (sk, pk, addr)
}

fn signed_tx(
    from_sk: &str,
    from_pk: &str,
    from_addr: &str,
    to: &str,
    amount: u64,
    fee: u64,
) -> Transaction {
    let mut tx = Transaction::new(from_addr, to, amount, fee);
    tx.sign(from_sk, from_pk).unwrap();
    tx
}

#[test]
fn genesis_is_deterministic_and_valid() {
    let g1 = Block::genesis();
    let g2 = Block::genesis();
    assert_eq!(g1.hash, g2.hash);
    assert_eq!(g1.index, 0);
    assert_eq!(g1.timestamp, config::GENESIS_TIMESTAMP);
    let bc = BlockChain::new();
    assert!(bc.is_chain_valid());
}

#[test]
fn mine_and_validate_chain() {
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    let block = bc.mine_block(&miner_addr);
    assert!(block.hash.starts_with("00"));
    assert_eq!(bc.chain.len(), 2);
    assert_eq!(bc.get_balance(&miner_addr), 16_000_000);
    assert!(bc.is_chain_valid());

    // Coinbase amount = reward + fees (fee = 0)
    assert_eq!(block.coinbase_tx.as_ref().unwrap().amount, 16_000_000);
}

#[test]
fn send_coins_and_double_spend_protection() {
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&addr_a); // A có 16 coins

    // Gửi 5 coins A -> B
    let tx = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 5_000_000, 0);
    bc.add_transaction(&tx).unwrap();
    assert_eq!(bc.mempool.len(), 1);
    bc.mine_block(&addr_b);
    assert_eq!(bc.get_balance(&addr_a), 11_000_000);
    assert_eq!(bc.get_balance(&addr_b), 16_000_000 + 5_000_000);

    // Double spend: cùng txid đã nằm trong block -> bị từ chối
    assert!(bc.add_transaction(&tx).is_err());

    // Vượt số dư
    let bad = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 999_000_000, 0);
    assert!(bc.add_transaction(&bad).is_err());
}

#[test]
fn pending_balance_blocks_mempool_overspend() {
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&addr_a);

    let tx1 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 10_000_000, 0);
    bc.add_transaction(&tx1).unwrap();
    // Đã pending 10 coins, chỉ còn 6 -> tx 7 coins phải fail
    let tx2 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 7_000_000, 0);
    assert!(bc.add_transaction(&tx2).is_err());
}

#[test]
fn receive_chain_only_if_longer() {
    let (_, _, miner_addr) = miner();
    let mut bc1 = BlockChain::with_difficulty(2);
    bc1.mine_block(&miner_addr);
    bc1.mine_block(&miner_addr);

    let mut bc2 = BlockChain::with_difficulty(2);
    assert!(bc2.receive_chain(&bc1.chain));
    assert_eq!(bc2.chain.len(), 3);
    assert_eq!(bc2.get_balance(&miner_addr), 32_000_000);

    // Chain ngắn hơn hoặc bằng -> từ chối
    assert!(!bc2.receive_chain(&bc1.chain[..2]));

    // Chain hỏng (sửa hash) -> từ chối
    let mut tampered = bc1.chain.clone();
    tampered[1].nonce += 1;
    assert!(!bc2.receive_chain(&tampered));
}

#[test]
fn receive_chain_from_higher_difficulty_peer() {
    // Node A mine ở difficulty 3; node B đang ở difficulty 2 —
    // chain của A vẫn phải được chấp nhận (difficulty suy từ chain,
    // không tin difficulty cục bộ của node nhận)
    let (_, _, miner_addr) = miner();
    let mut bc_a = BlockChain::with_difficulty(3);
    bc_a.mine_block(&miner_addr);
    bc_a.mine_block(&miner_addr);

    let mut bc_b = BlockChain::with_difficulty(2);
    assert!(bc_b.receive_chain(&bc_a.chain));
    assert_eq!(bc_b.chain.len(), 3);
    // Difficulty cục bộ của B sync theo tip mới của A
    assert_eq!(bc_b.difficulty, 3);

    // Ngược lại: block có PoW yếu hơn tip -> từ chối
    let (_, _, weak_miner) = miner();
    let mut bc_weak = BlockChain::with_difficulty(1);
    bc_weak.mine_block(&weak_miner);
    // Block #1 của bc_weak chỉ có 1 số 0 đầu — không đủ difficulty 2 của bc_b
    assert!(!bc_b.receive_block(&bc_weak.chain[1]));
}

#[test]
fn pow_difficulty_drops_rejected_in_chain() {
    // Chain: genesis + 2 block difficulty 2, rồi block chỉ đạt difficulty 1
    // -> chain bị từ chối (PoW không được yếu hơn block trước)
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&miner_addr);
    bc.mine_block(&miner_addr);

    // Mine block #3 ở difficulty 1, thử lại cho đến khi hash chỉ có đúng
    // 1 số 0 đầu (không tình cờ đạt 2+ như difficulty 2)
    let weak;
    loop {
        let (mut block, _) = bc.prepare_block(&miner_addr);
        block.mine_block(1, &miner_addr);
        if !block.hash.starts_with("00") {
            weak = block;
            break;
        }
        // hash tình cờ đạt 2 số 0 — rollback mempool rồi mine lại
        bc.mempool.clear();
    }
    assert!(weak.hash.starts_with('0'));

    let mut chain = bc.chain.clone();
    chain.push(weak);
    assert!(!BlockChain::new().receive_chain(&chain));
}

#[test]
fn difficulty_decrease_propagates_across_network() {
    // Retarget đầu tiên đo từ genesis (ts cố định 2021) -> window "quá chậm"
    // -> difficulty giảm 2->1 cho block #11. Mạng phải chấp nhận difficulty
    // GIẢM tại boundary (trước đây mọi block yếu hơn tip đều bị từ chối).
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    for _ in 0..10 {
        bc.mine_block(&miner_addr);
    }
    assert_eq!(bc.difficulty, 1, "retarget ở block #10 phải giảm difficulty");
    bc.mine_block(&miner_addr); // block #11 mine ở difficulty 1
    assert_eq!(bc.get_latest_block().difficulty, 1);
    assert!(bc.is_chain_valid());

    // Peer sync cả chain
    let mut peer = BlockChain::new();
    assert!(peer.receive_chain(&bc.chain));

    // Peer nhận từng block — block #11 yếu hơn tip #10 vẫn được chấp nhận
    let mut peer2 = BlockChain::new();
    for b in &bc.chain[1..] {
        assert!(peer2.receive_block(b), "block #{} phải được chấp nhận", b.index);
    }
}

#[test]
fn difficulty_change_outside_retarget_rejected() {
    // Đổi difficulty giữa interval (không phải boundary) -> chain từ chối
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&miner_addr); // block #1 (d2) — index 1 không phải boundary

    let (mut block, _) = bc.prepare_block(&miner_addr);
    block.mine_block(3, &miner_addr); // tự ý tăng lên 3
    let mut chain = bc.chain.clone();
    chain.push(block);
    assert!(!BlockChain::new().receive_chain(&chain));
}

#[test]
fn block_without_coinbase_rejected() {
    // Block hợp lệ mọi thứ (PoW, linkage, merkle) nhưng không có coinbase
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(1);
    let (mut block, _) = bc.prepare_block(&miner_addr);
    block.coinbase_tx = None;
    block.merkle_root = Some(block.calculate_merkle_root());
    block.difficulty = 1;
    let target = "0".repeat(1);
    loop {
        block.nonce += 1;
        block.hash = block.calculate_hash();
        if block.hash.starts_with(&target) {
            break;
        }
    }
    let mut victim = BlockChain::with_difficulty(1);
    assert!(
        !victim.receive_block(&block),
        "block không coinbase phải bị từ chối"
    );
}

#[test]
fn tx_from_mismatched_public_key_rejected() {
    // Kẻ tấn công ký tx "từ" địa chỉ nạn nhân bằng key của chính mình —
    // chữ ký verify được (không khớp pk nào của nạn nhân cả) nhưng from
    // không phải địa chỉ suy từ pk đã ký -> phải bị từ chối
    let (_, _, victim) = miner();
    let (evil_sk, evil_pk, evil_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&victim);

    let mut tx = Transaction::new(&victim, &evil_addr, 15_000_000, 0);
    tx.sign(&evil_sk, &evil_pk).unwrap();
    assert!(
        bc.add_transaction(&tx).is_err(),
        "tx ghi nợ địa chỉ người khác phải bị từ chối"
    );

    // Block chứa tx như vậy cũng bị từ chối ở receive path
    let mut evil_block = bc.prepare_block(&evil_addr).0;
    evil_block.transactions = vec![tx];
    evil_block.mine_block(2, &evil_addr);
    assert!(
        !bc.receive_block(&evil_block),
        "block chứa tx from-lệch-pk phải bị từ chối"
    );
}

#[test]
fn tx_without_txid_rejected() {
    // Tx hợp lệ nhưng bị strip txid -> không được vào mempool (nếu vào được,
    // prepare_block sẽ chọn nó lại mỗi block -> debit lặp + node khác từ chối)
    let (sk, pk, addr) = miner();
    let mut bc = BlockChain::with_difficulty(1);
    bc.mine_block(&addr);
    let mut tx = signed_tx(&sk, &pk, &addr, &"b".repeat(64), 1_000_000, 0);
    tx.txid = None;
    assert!(bc.add_transaction(&tx).is_err());
}

#[test]
fn fake_coinbase_type_tx_rejected() {
    // tx_type = "COINBASE" tự khai -> bypass chữ ký + số dư nếu được chấp nhận
    let (_, _, victim) = miner();
    let evil = "e".repeat(64);
    let mut bc = BlockChain::with_difficulty(1);
    bc.mine_block(&victim);
    let mut tx = Transaction::new(&evil, &evil, 1_000_000_000, 0);
    tx.tx_type = crate::blockchain::transaction::TYPE_COINBASE.to_string();
    assert!(bc.add_transaction(&tx).is_err());
}

#[test]
fn receive_block_validates_linkage() {
    let (_, _, miner_addr) = miner();
    let mut bc1 = BlockChain::with_difficulty(2);
    bc1.mine_block(&miner_addr);
    let block1 = bc1.chain[1].clone();

    let mut bc2 = BlockChain::with_difficulty(2);
    assert!(bc2.receive_block(&block1));
    assert_eq!(bc2.chain.len(), 2);

    // Block trùng index -> fail
    assert!(!bc2.receive_block(&block1));
}

#[test]
fn estimate_fee_math() {
    let mut bc = BlockChain::new();
    assert_eq!(bc.estimate_fee(), 0);
    bc.mempool.push(Transaction::new(
        &"a".repeat(64),
        &"b".repeat(64),
        1,
        1_000_000,
    ));
    bc.mempool.push(Transaction::new(
        &"a".repeat(64),
        &"b".repeat(64),
        1,
        2_000_000,
    ));
    // avg = 1.5, *1.1 = 1.65 -> ceil = 1_650_000
    assert_eq!(bc.estimate_fee(), 1_650_000);
}

#[test]
fn halving_via_mining() {
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(1);
    // Mine đến block 50 (index 50) — reward phải halving còn 8 coins.
    // ponytail: trước mỗi lần mine, lùi timestamp tip về 1 adjustment-interval
    // (10 block * 30s) để adjust_difficulty thấy tổng thời gian ≈ target —
    // difficulty đứng yên ở 1. Không làm vậy thì difficulty leo lên max 6
    // và test (debug build) mine 16M hash/block, quá chậm.
    let back = (config::DIFFICULTY_ADJUSTMENT_INTERVAL as u64) * config::TARGET_BLOCK_TIME;
    while bc.chain.len() <= config::HALVING_INTERVAL {
        {
            let tip = bc.chain.last_mut().unwrap();
            tip.timestamp = util::now_ms().saturating_sub(back);
            tip.hash = tip.calculate_hash();
        }
        bc.mine_block(&miner_addr);
    }
    let last = bc.get_latest_block();
    assert_eq!(last.coinbase_tx.as_ref().unwrap().amount, 8_000_000);
    assert_eq!(bc.get_block_reward(), 8_000_000);
}

#[test]
fn replay_after_receive_block_rejected() {
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc1 = BlockChain::with_difficulty(2);
    bc1.mine_block(&addr_a);

    let tx = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 1_000_000, 0);
    bc1.add_transaction(&tx).unwrap();
    bc1.mine_block(&addr_b);
    let block_with_tx = bc1.chain.last().unwrap().clone();

    // Peer nhận block qua mạng rồi bị replay cùng tx
    let mut bc2 = BlockChain::with_difficulty(2);
    bc2.receive_block(&bc1.chain[1]);
    assert!(bc2.receive_block(&block_with_tx));
    assert!(
        bc2.add_transaction(&tx).is_err(),
        "replay tx đã confirm qua receive_block phải bị từ chối"
    );
}

#[test]
fn replay_after_receive_chain_rejected() {
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc1 = BlockChain::with_difficulty(2);
    bc1.mine_block(&addr_a);

    let tx = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 1_000_000, 0);
    bc1.add_transaction(&tx).unwrap();
    bc1.mine_block(&addr_b);

    // Peer sync chain dài hơn rồi bị replay cùng tx
    let mut bc2 = BlockChain::with_difficulty(2);
    assert!(bc2.receive_chain(&bc1.chain));
    assert!(
        bc2.add_transaction(&tx).is_err(),
        "replay tx đã confirm qua receive_chain phải bị từ chối"
    );
}

#[test]
fn prepare_mine_apply_flow() {
    let (_, _, miner_addr) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    let (mut block, difficulty) = bc.prepare_block(&miner_addr);
    block.mine_block(difficulty, &miner_addr);
    assert!(bc.apply_mined_block(&block));
    assert_eq!(bc.chain.len(), 2);
    assert_eq!(bc.get_balance(&miner_addr), 16_000_000);
    assert!(bc.is_chain_valid());
}

#[test]
fn apply_rejects_when_chain_moved_and_restores_mempool() {
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&addr_a);

    let tx = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 1_000_000, 0);
    bc.add_transaction(&tx).unwrap();

    // Prepare (tx bị remove khỏi mempool), rồi chain đổi trước khi apply
    let (mut block, difficulty) = bc.prepare_block(&addr_b);
    assert_eq!(block.transactions.len(), 1);
    assert!(bc.mempool.is_empty());
    block.mine_block(difficulty, &addr_b);
    bc.mine_block(&addr_a); // peer thêm block khác -> chain moved

    assert!(!bc.apply_mined_block(&block));
    assert_eq!(bc.chain.len(), 3); // block bị bỏ không được thêm
                                   // Tx được trả về mempool
    assert!(bc.mempool.iter().any(|m| m.txid == tx.txid));
}

#[test]
fn apply_rejects_stale_overspending_tx() {
    // Tx hợp lệ vào mempool; block chứa nó được mine nhưng TRƯỚC khi apply,
    // một block khác đã tiêu hết số dư của người gửi -> block chứa tx vượt
    // số dư phải bị từ chối, không được debit âm.
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut bc = BlockChain::with_difficulty(2);
    bc.mine_block(&addr_a); // A có 16 coins

    // A gửi B 10 coins (tx1) — block chứa tx1 được prepare
    let tx1 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 10_000_000, 0);
    bc.add_transaction(&tx1).unwrap();
    let (mut block1, difficulty) = bc.prepare_block(&addr_b);
    block1.mine_block(difficulty, &addr_b);
    // block1 chưa apply — nằm chờ

    // Trong lúc đó A gửi B 10 coins nữa (tx2) — số dư vẫn 16 (tx1 chưa confirm)
    let tx2 = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 10_000_000, 0);
    bc.add_transaction(&tx2).unwrap();
    let mut block2 = bc.prepare_block(&addr_a).0;
    block2.mine_block(difficulty, &addr_a);
    assert!(bc.apply_mined_block(&block2)); // block2 confirm tx2, A còn 6 coins

    // Apply block1 giờ vượt số dư: A còn 6, tx1 cần 10
    let balance_before = bc.get_balance(&addr_a);
    assert!(
        !bc.apply_mined_block(&block1),
        "block chứa tx vượt số dư phải bị từ chối"
    );
    assert_eq!(bc.get_balance(&addr_a), balance_before);
    assert_eq!(bc.chain.len(), 3); // block1 không được thêm
}

#[test]
fn receive_block_rejects_overspending_tx() {
    // Node độc hại gửi block nối tiếp tip hợp lệ nhưng chứa tx VƯỢT số dư
    // (không ký / không đủ tiền) — receive_block phải validate từng tx,
    // không chỉ header của block.
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut honest = BlockChain::with_difficulty(2);
    honest.mine_block(&addr_a); // A có 16 coins

    // Kẻ tấn công tự dựng block #2 nối tiếp tip, chứa tx A->B 100 coins
    // (A chỉ có 16) với chữ ký hợp lệ
    let mut evil = honest.prepare_block(&addr_b).0;
    let overspend = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 100_000_000, 0);
    evil.transactions = vec![overspend];
    evil.mine_block(2, &addr_b);

    // Block nhận từ mạng: hash/PoW/linkage hợp lệ nhưng tx vượt số dư
    let mut victim = BlockChain::with_difficulty(2);
    assert!(victim.receive_block(&honest.chain[1]), "block #1 hợp lệ");
    assert!(
        !victim.receive_block(&evil),
        "block chứa tx vượt số dư phải bị từ chối"
    );
    assert_eq!(victim.chain.len(), 2); // evil block không được thêm
}

#[test]
fn receive_block_rejects_unsigned_tx() {
    // Block chứa tx KHÔNG ký (sửa amount sau khi ký làm chữ ký lệch)
    let (sk_a, pk_a, addr_a) = miner();
    let (_, _, addr_b) = miner();
    let mut honest = BlockChain::with_difficulty(2);
    honest.mine_block(&addr_a);

    let mut tampered = signed_tx(&sk_a, &pk_a, &addr_a, &addr_b, 1_000_000, 0);
    tampered.amount = 2_000_000; // sửa amount -> chữ ký không còn khớp
    let mut evil = honest.prepare_block(&addr_b).0;
    evil.transactions = vec![tampered];
    evil.mine_block(2, &addr_b);

    let mut victim = BlockChain::with_difficulty(2);
    assert!(victim.receive_block(&honest.chain[1]));
    assert!(
        !victim.receive_block(&evil),
        "block chứa tx chữ ký không hợp lệ phải bị từ chối"
    );
}

