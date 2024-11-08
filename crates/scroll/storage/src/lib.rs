//! Scroll storage implementation.

use alloy_primitives::{map::Entry, Address, B256, U256};
use reth_revm::{
    database::EvmStateProvider,
    primitives::{AccountInfo, Bytecode},
    Database,
};
use reth_scroll_revm::primitives::{poseidon, ScrollPostExecutionContext, POSEIDON_EMPTY};
use reth_storage_errors::provider::ProviderError;

/// A similar construct as `StateProviderDatabase` which captures additional Scroll context derived
/// from bytecode during execution.
#[derive(Clone, Debug)]
pub struct ScrollStateProviderDatabase<DB> {
    /// Scroll post execution context.
    post_execution_context: ScrollPostExecutionContext,
    /// The database.
    pub db: DB,
}

impl<DB> ScrollStateProviderDatabase<DB> {
    /// Creates a [`ScrollStateProviderDatabase`] from the provided DB.
    pub fn new(db: DB) -> Self {
        Self { db, post_execution_context: Default::default() }
    }

    /// Consumes the provider and returns the post execution context.
    pub fn post_execution_context(self) -> ScrollPostExecutionContext {
        self.post_execution_context
    }
}

impl<DB: EvmStateProvider> Database for ScrollStateProviderDatabase<DB> {
    type Error = ProviderError;

    /// Retrieves basic account information for a given address.
    ///
    /// Returns `Ok` with `Some(AccountInfo)` if the account exists,
    /// `None` if it doesn't, or an error if encountered.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let Some(account) = self.db.basic_account(address)? else { return Ok(None) };
        let Some(code_hash) = account.bytecode_hash else { return Ok(Some(account.into())) };

        if let Entry::Vacant(entry) = self.post_execution_context.entry(code_hash) {
            let code = self.db.bytecode_by_hash(code_hash)?.unwrap_or_default();
            let poseidon_hash =
                if code.is_empty() { POSEIDON_EMPTY } else { poseidon(code.bytecode()) };
            entry.insert((code.len(), poseidon_hash));
        }

        Ok(Some(account.into()))
    }

    /// Retrieves the bytecode associated with a given code hash.
    ///
    /// Returns `Ok` with the bytecode if found, or the default bytecode otherwise.
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.db.bytecode_by_hash(code_hash)?.unwrap_or_default().0)
    }

    /// Retrieves the storage value at a specific index for a given address.
    ///
    /// Returns `Ok` with the storage value, or the default value if not found.
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self.db.storage(address, B256::new(index.to_be_bytes()))?.unwrap_or_default())
    }

    /// Retrieves the block hash for a given block number.
    ///
    /// Returns `Ok` with the block hash if found, or the default hash otherwise.
    /// Note: It safely casts the `number` to `u64`.
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        Ok(self.db.block_hash(number)?.unwrap_or_default())
    }
}
