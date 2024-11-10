use revm::primitives::{b256, B256};

/// The Poseidon hash of the empty string `""`.
pub const POSEIDON_EMPTY: B256 =
    b256!("2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864");

/// Poseidon code hash
pub fn poseidon(code: &[u8]) -> B256 {
    poseidon_bn254::hash_code(code).into()
}
