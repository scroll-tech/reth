//! Standalone crate for Scroll-specific Reth primitive types.

use alloy_primitives::{B256, U256};
use reth_trie::TrieAccount;

/// Poseidon hashing primitives.
pub mod poseidon;

/// A Scroll account as represented in the trie.
#[derive(Debug)]
pub struct ScrollTrieAccount {
    /// nonce
    pub nonce: u64,
    /// code size
    pub code_size: u64,
    /// balance
    pub balance: U256,
    /// storage root
    pub storage_root: B256,
    /// keccak code hash
    pub code_hash: B256,
    /// poseidon code hash
    pub poseidon_code_hash: B256,
}

// TODO: Temporary method to convert from standard ethereum `TrieAccount` to `ScrollTrieAccount`
// TODO: Fix cast
impl From<TrieAccount> for ScrollTrieAccount {
    fn from(value: TrieAccount) -> Self {
        ScrollTrieAccount {
            // TODO(frisitano): introduce code size and poseidon code hash following integration
            // with Account changes
            poseidon_code_hash: Default::default(),
            code_size: Default::default(),
            nonce: value.nonce,
            balance: value.balance,
            // TODO(frisitano): introduce storage root
            storage_root: Default::default(),
            code_hash: value.code_hash,
        }
    }
}
