use alloy_primitives::B256;
use scroll_primitives::poseidon::{split_and_hash_be_bytes, PrimeField, FIELD_ELEMENT_REPR_BYTES};

// TODO(frisitano): Implement `KeyHasher` trait from upstream. Also consider introducing a
// `HashingScheme` trait that combines both `KeyHasher` and `ValueHasher` traits via GATs.

/// An implementation of a key hasher that uses Poseidon.
#[derive(Debug)]
pub struct PoseidonKeyHasher;

impl PoseidonKeyHasher {
    /// Hashes the key using the Poseidon hash function.
    ///
    /// The bytes are expected to be provided in big endian format.
    ///
    /// Panics if the number of bytes provided is greater than the number of bytes in the
    /// binary representation of a field element (32).
    ///
    /// Returns the hash digest in little endian representation with bits reversed.
    pub fn hash_key<T: AsRef<[u8]>>(bytes: T) -> B256 {
        debug_assert!(bytes.as_ref().len() <= FIELD_ELEMENT_REPR_BYTES);
        let mut bytes = split_and_hash_be_bytes(bytes.as_ref()).to_repr();
        bytes.iter_mut().for_each(|byte| *byte = byte.reverse_bits());
        bytes.into()
    }
}
