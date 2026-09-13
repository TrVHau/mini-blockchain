//! Merkle Tree — port trung thực từ src/util/MerkleTree.js.
//! Lưu ý: hash_pair nối CHUỖI HEX (ASCII), không phải bytes.

use crate::crypto::sha256_hex;

pub fn hash(data: &str) -> String {
    sha256_hex(data.as_bytes())
}

pub fn hash_pair(left: &str, right: &str) -> String {
    hash(&format!("{left}{right}"))
}

fn pad_odd(level: &mut Vec<String>) {
    if level.len() % 2 != 0 {
        if let Some(last) = level.last() {
            let last = last.clone();
            level.push(last);
        }
    }
}

/// Merkle root từ danh sách tx hashes.
/// - rỗng -> sha256("empty")
/// - 1 phần tử -> chính nó
/// - số lẻ -> duplicate phần tử cuối ở mỗi level
pub fn calculate_root(tx_hashes: &[String]) -> String {
    if tx_hashes.is_empty() {
        return hash("empty");
    }
    if tx_hashes.len() == 1 {
        return tx_hashes[0].clone();
    }

    let mut level: Vec<String> = tx_hashes.to_vec();
    pad_odd(&mut level);

    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len() / 2);
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            let right = level.get(i + 1).unwrap_or(left);
            next_level.push(hash_pair(left, right));
            i += 2;
        }
        level = next_level;
        pad_odd(&mut level);
    }

    level.into_iter().next().unwrap()
}

/// Một node trong Merkle proof
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)] // API công khai cho Merkle proof (dùng trong test + verify_transaction)
pub struct ProofNode {
    pub hash: String,
    /// "left" | "right" — vị trí của node anh em so với node hiện tại
    pub position: String,
}

/// Proof path cho tx tại `index`
#[allow(dead_code)] // API công khai cho Merkle proof
pub fn get_proof(tx_hashes: &[String], index: usize) -> Option<Vec<ProofNode>> {
    if tx_hashes.is_empty() || index >= tx_hashes.len() {
        return None;
    }

    let mut proof = Vec::new();
    let mut level: Vec<String> = tx_hashes.to_vec();
    pad_odd(&mut level);

    let mut current_index = index;

    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len() / 2);
        let mut i = 0;
        while i < level.len() {
            let left = &level[i];
            let right = level.get(i + 1).unwrap_or(left);

            if i == current_index || i + 1 == current_index {
                let is_left = current_index % 2 == 0;
                proof.push(ProofNode {
                    hash: if is_left { right.clone() } else { left.clone() },
                    position: if is_left { "right" } else { "left" }.to_string(),
                });
            }

            next_level.push(hash_pair(left, right));
            i += 2;
        }

        level = next_level;
        current_index /= 2;
        pad_odd(&mut level);
    }

    Some(proof)
}

/// Verify tx hash thuộc về tree với merkle root
#[allow(dead_code)] // API công khai cho Merkle proof
pub fn verify_proof(tx_hash: &str, proof: &[ProofNode], root: &str) -> bool {
    let mut current = tx_hash.to_string();
    for node in proof {
        current = if node.position == "left" {
            hash_pair(&node.hash, &current)
        } else {
            hash_pair(&current, &node.hash)
        };
    }
    current == root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hashes(n: usize) -> Vec<String> {
        (0..n).map(|i| hash(&i.to_string())).collect()
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(calculate_root(&[]), hash("empty"));
        let one = hashes(1);
        assert_eq!(calculate_root(&one), one[0]);
    }

    #[test]
    fn proof_verifies() {
        for n in [2, 3, 4, 5, 7, 8] {
            let leaves = hashes(n);
            let root = calculate_root(&leaves);
            for i in 0..n {
                let proof = get_proof(&leaves, i).unwrap();
                assert!(verify_proof(&leaves[i], &proof, &root), "n={n} i={i}");
            }
        }
    }

    #[test]
    fn proof_rejects_wrong_hash() {
        let leaves = hashes(4);
        let root = calculate_root(&leaves);
        let proof = get_proof(&leaves, 2).unwrap();
        assert!(!verify_proof(&leaves[0], &proof, &root));
    }
}
