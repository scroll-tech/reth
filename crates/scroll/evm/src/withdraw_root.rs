use alloy_primitives::{address, Address, U256};
use revm::{database::State, Database};

const L2_MESSAGE_QUEUE_ADDRESS: Address = address!("0x5300000000000000000000000000000000000000");
const WITHDRAW_TRIE_ROOT_SLOT: U256 = U256::ZERO;

/// Instance that implements the trait can load the `L2MessageQueue` withdraw root in state.
pub trait LoadWithdrawRoot<DB: Database> {
    /// Load the withdrawal root.
    fn load_withdraw_root(&mut self) -> Result<(), DB::Error>;
}

impl<DB: Database> LoadWithdrawRoot<DB> for State<DB> {
    fn load_withdraw_root(&mut self) -> Result<(), DB::Error> {
        // we load the account in cache and query the storage slot. The storage slot will only be
        // loaded from database if it is not already know.
        self.load_cache_account(L2_MESSAGE_QUEUE_ADDRESS)?;
        let _ = revm::Database::storage(self, L2_MESSAGE_QUEUE_ADDRESS, WITHDRAW_TRIE_ROOT_SLOT);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, convert::Infallible};

    use alloy_primitives::B256;
    use revm::{bytecode::Bytecode, state::AccountInfo};
    use revm_primitives::{StorageKey, StorageValue};

    #[derive(Default)]
    struct InMemoryDb {
        pub accounts: HashMap<Address, AccountInfo>,
        pub storage: HashMap<(Address, U256), U256>,
    }

    impl Database for InMemoryDb {
        type Error = Infallible;

        fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
            Ok(self.accounts.get(&address).cloned())
        }

        fn code_by_hash(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
            Ok(Default::default())
        }

        fn storage(
            &mut self,
            address: Address,
            index: StorageKey,
        ) -> Result<StorageValue, Self::Error> {
            Ok(self.storage.get(&(address, index)).copied().unwrap_or_default())
        }

        fn block_hash(&mut self, _number: u64) -> Result<B256, Self::Error> {
            Ok(Default::default())
        }
    }

    #[test]
    fn test_should_load_withdraw_root() -> eyre::Result<()> {
        // init db
        let mut db = InMemoryDb::default();

        // load L2 message queue contract
        let withdraw_root = U256::random();
        db.accounts.insert(L2_MESSAGE_QUEUE_ADDRESS, Default::default());
        db.storage.insert((L2_MESSAGE_QUEUE_ADDRESS, U256::ZERO), withdraw_root);

        let mut state =
            State::builder().with_database(db).with_bundle_update().without_state_clear().build();

        assert!(state
            .cache
            .accounts
            .get(&L2_MESSAGE_QUEUE_ADDRESS)
            .map(|acc| acc.storage_slot(WITHDRAW_TRIE_ROOT_SLOT))
            .is_none());

        // load root
        state.load_withdraw_root()?;

        assert_eq!(
            state
                .cache
                .accounts
                .get(&L2_MESSAGE_QUEUE_ADDRESS)
                .unwrap()
                .storage_slot(WITHDRAW_TRIE_ROOT_SLOT)
                .unwrap(),
            withdraw_root
        );

        Ok(())
    }
}
