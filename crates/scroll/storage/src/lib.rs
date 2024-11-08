//! Scroll storage implementation.

use alloy_primitives::{map::Entry, Address, B256, U256};
use reth_revm::{
    database::EvmStateProvider,
    primitives::{AccountInfo, Bytecode},
    Database,
};
use reth_scroll_primitives::ScrollPostExecutionContext;
use reth_storage_errors::provider::ProviderError;

/// A similar construct as `StateProviderDatabase` which captures additional Scroll context for
/// touched accounts during execution.
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
    /// Caches the Scroll context for the touched account if it
    /// has bytecode.
    ///
    /// Returns `Ok` with `Some(AccountInfo)` if the account exists,
    /// `None` if it doesn't, or an error if encountered.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let Some(account) = self.db.basic_account(address)? else { return Ok(None) };
        let Some(code_hash) = account.bytecode_hash else { return Ok(Some(account.into())) };

        let bytecode_context = (account.code_size, account.poseidon_code_hash);
        if let Entry::Vacant(entry) = self.post_execution_context.entry(code_hash) {
            entry.insert(bytecode_context);
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
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        Ok(self.db.block_hash(number)?.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use crate::ScrollStateProviderDatabase;
    use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
    use reth_primitives_traits::Account;
    use reth_revm::{test_utils::StateProviderTest, Database};
    use reth_scroll_primitives::{poseidon, POSEIDON_EMPTY};

    #[test]
    fn test_scroll_post_execution_context() -> eyre::Result<()> {
        let mut db = StateProviderTest::default();

        // insert an eoa in the db
        let eoa_address = Address::random();
        let eoa = Account {
            nonce: 0,
            balance: U256::MAX,
            bytecode_hash: None,
            code_size: 0,
            poseidon_code_hash: B256::ZERO,
        };
        db.insert_account(eoa_address, eoa, None, Default::default());

        // insert a contract account in the db
        let contract_address = Address::random();
        let bytecode = Bytes::copy_from_slice(&[0x0, 0x1, 0x2, 0x3, 0x4, 0x5]);
        let bytecode_hash = keccak256(&bytecode);
        let poseidon_code_hash = poseidon(&bytecode);
        let contract = Account {
            nonce: 0,
            balance: U256::MAX,
            bytecode_hash: Some(bytecode_hash),
            code_size: bytecode.len() as u64,
            poseidon_code_hash,
        };
        db.insert_account(contract_address, contract, Some(bytecode.clone()), Default::default());

        // insert an empty contract account in the db
        let empty_contract_address = Address::random();
        let empty_bytecode = Bytes::copy_from_slice(&[]);
        let empty_bytecode_hash = keccak256(&empty_bytecode);
        let empty_contract = Account {
            nonce: 0,
            balance: U256::MAX,
            bytecode_hash: None,
            code_size: 0,
            poseidon_code_hash: POSEIDON_EMPTY,
        };
        db.insert_account(
            empty_contract_address,
            empty_contract,
            Some(empty_bytecode),
            Default::default(),
        );

        let mut provider = ScrollStateProviderDatabase::new(db);

        // check eoa is in db
        let _ = provider.basic(eoa_address)?.unwrap();
        // check contract is in db
        let _ = provider.basic(contract_address)?.unwrap();
        // check empty contract is in db
        let _ = provider.basic(empty_contract_address)?.unwrap();

        // check provider context contains only contract and empty contract
        let post_execution_context = provider.post_execution_context();
        assert_eq!(post_execution_context.len(), 2);

        // check post execution context is correct for contract
        let (code_size, poseidon_code_hash) = post_execution_context.get(&bytecode_hash).unwrap();
        assert_eq!(*code_size, 6);
        assert_eq!(*poseidon_code_hash, poseidon(&bytecode));

        // check post execution context is correct for empty contract
        let (code_size, poseidon_code_hash) =
            post_execution_context.get(&empty_bytecode_hash).unwrap();
        assert_eq!(*code_size, 0);
        assert_eq!(*poseidon_code_hash, POSEIDON_EMPTY);

        Ok(())
    }
}
