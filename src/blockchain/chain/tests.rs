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
