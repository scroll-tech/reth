//! The implementation of scrolls binary Merkle Patricia Trie used a cryptographic state commitment.

mod root;
pub use root::{StateRoot, StorageRoot};

mod key;
mod value;

// RE-EXPORTS
pub use key::PoseidonKeyHasher;
pub use value::PosiedonValueHasher;
