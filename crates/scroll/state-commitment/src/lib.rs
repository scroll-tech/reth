//! The implementation of scrolls binary Merkle Patricia Trie used a cryptographic state commitment.

mod root;
pub use root::{StateRoot, StorageRoot};

mod key;
mod value;

/// test utils for the state commitment
#[cfg(feature = "test-utils")]
pub mod test_utils;

// RE-EXPORTS
pub use key::PoseidonKeyHasher;
pub use value::PosiedonValueHasher;
