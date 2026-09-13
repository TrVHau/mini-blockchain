//! Crypto helpers: SHA-256 hex, ECDSA secp256k1 sign/verify, address derivation.
//! Thay cho `node:crypto` của bản JS: keys lưu hex thay PEM.

use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

/// SHA-256 của bytes -> hex string
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

/// Địa chỉ = sha256(public key bytes) hex (64 ký tự) — tương đương
/// publicKeyToAddress(PEM) của JS nhưng trên bytes public key nén.
pub fn public_key_to_address(public_key: &PublicKey) -> String {
    sha256_hex(&public_key.serialize())
}

/// Một cặp khóa mới: (private_key_hex 64 chars, public_key_hex compressed 66 chars)
pub fn generate_keypair() -> (String, String) {
    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (hex::encode(secret_key.secret_bytes()), hex::encode(public_key.serialize()))
}

/// Ký message (bytes) bằng private key hex -> DER signature hex
pub fn sign(private_key_hex: &str, message: &[u8]) -> Result<String, String> {
    let secp = Secp256k1::signing_only();
    let sk_bytes = hex::decode(private_key_hex)
        .map_err(|_| "Invalid private key hex".to_string())?;
    let secret_key = SecretKey::from_slice(&sk_bytes)
        .map_err(|_| "Invalid private key".to_string())?;
    // secp256k1 crate yêu cầu message đúng 32 bytes
    let msg = Message::from_digest_slice(&Sha256::digest(message))
        .map_err(|_| "Hash failed".to_string())?;
    let sig = secp.sign_ecdsa(&msg, &secret_key);
    Ok(hex::encode(sig.serialize_der()))
}

/// Verify signature hex (DER) trên message bằng public key hex (compressed)
pub fn verify(public_key_hex: &str, message: &[u8], signature_hex: &str) -> bool {
    let inner = || -> Result<bool, String> {
        let secp = Secp256k1::verification_only();
        let pk_bytes = hex::decode(public_key_hex)
            .map_err(|_| "Invalid public key hex".to_string())?;
        let public_key =
            PublicKey::from_slice(&pk_bytes).map_err(|_| "Invalid public key".to_string())?;
        let sig_bytes = hex::decode(signature_hex)
            .map_err(|_| "Invalid signature hex".to_string())?;
        let signature = Signature::from_der(&sig_bytes)
            .map_err(|_| "Invalid signature DER".to_string())?;
        let msg = Message::from_digest_slice(&Sha256::digest(message))
            .map_err(|_| "Hash failed".to_string())?;
        Ok(secp.verify_ecdsa(&msg, &signature, &public_key).is_ok())
    };
    inner().unwrap_or(false)
}

/// Derive public key hex (compressed) từ private key hex
pub fn private_to_public(private_key_hex: &str) -> Result<String, String> {
    let secp = Secp256k1::new();
    let sk_bytes =
        hex::decode(private_key_hex).map_err(|_| "Invalid private key hex".to_string())?;
    let secret_key =
        SecretKey::from_slice(&sk_bytes).map_err(|_| "Invalid private key".to_string())?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    Ok(hex::encode(public_key.serialize()))
}

/// Địa chỉ từ public key hex (dùng cho import wallet)
pub fn address_from_public_hex(public_key_hex: &str) -> Result<String, String> {
    let pk_bytes =
        hex::decode(public_key_hex).map_err(|_| "Invalid public key hex".to_string())?;
    let public_key =
        PublicKey::from_slice(&pk_bytes).map_err(|_| "Invalid public key".to_string())?;
    Ok(public_key_to_address(&public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        // sha256("abc")
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sign_verify_roundtrip() {
        let (sk, pk) = generate_keypair();
        let msg = b"hello blockchain";
        let sig = sign(&sk, msg).unwrap();
        assert!(verify(&pk, msg, &sig));
        assert!(!verify(&pk, b"tampered", &sig));
    }

    #[test]
    fn private_to_public_matches_generated() {
        let (sk, pk) = generate_keypair();
        assert_eq!(private_to_public(&sk).unwrap(), pk);
        let addr = address_from_public_hex(&pk).unwrap();
        assert_eq!(addr.len(), 64);
        assert!(addr.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
