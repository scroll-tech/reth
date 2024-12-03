//! Primitive types for the Scroll extension of `Reth`.

#![warn(unused_crate_dependencies)]

use alloy_primitives::{B256, U256};
use reth_trie::TrieAccount;

pub use execution_context::ScrollPostExecutionContext;
mod execution_context;

pub use account_extension::AccountExtension;
mod account_extension;

pub use l1_transaction::{
    ScrollL1MessageTransactionFields, TxL1Message, L1_MESSAGE_TRANSACTION_TYPE,
};
pub mod l1_transaction;

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
            storage_root: value.storage_root,
            code_hash: value.code_hash,
        }
    }
}
