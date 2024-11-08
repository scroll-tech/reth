use super::bundle_account::ScrollBundleAccount;
use crate::{
    primitives::{ScrollAccountInfo, ScrollPostExecutionContext},
    states::{
        changes::{ScrollPlainStateReverts, ScrollStateChangeset},
        reverts::ScrollReverts,
    },
};
use revm::{
    db::{states::PlainStorageChangeset, BundleState, OriginalValuesKnown},
    primitives::{map::HashMap, Address, Bytecode, B256, KECCAK_EMPTY},
};

/// An equivalent of the [`BundleState`] modified with Scroll compatible fields.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ScrollBundleState {
    /// Account state.
    pub state: HashMap<Address, ScrollBundleAccount>,
    /// All created contracts in this block.
    pub contracts: HashMap<B256, Bytecode>,
    /// Changes to revert.
    ///
    /// Note: Inside vector is *not* sorted by address.
    /// But it is unique by address.
    pub reverts: ScrollReverts,
    /// The size of the plain state in the bundle state.
    pub state_size: usize,
    /// The size of reverts in the bundle state.
    pub reverts_size: usize,
}

impl From<(BundleState, ScrollPostExecutionContext)> for ScrollBundleState {
    fn from((bundle, context): (BundleState, ScrollPostExecutionContext)) -> Self {
        let reverts = bundle
            .reverts
            .iter()
            .map(|reverts| {
                reverts
                    .iter()
                    .map(|(add, revert)| (*add, (revert.clone(), &context).into()))
                    .collect()
            })
            .collect();

        let state = bundle
            .state
            .into_iter()
            .map(|(add, account)| (add, (account, &context).into()))
            .collect();

        Self {
            state,
            contracts: bundle.contracts,
            reverts: ScrollReverts::new(reverts),
            state_size: bundle.state_size,
            reverts_size: bundle.reverts_size,
        }
    }
}

impl ScrollBundleState {
    /// Returns the approximate size of changes in the bundle state.
    /// The estimation is not precise, because the information about the number of
    /// destroyed entries that need to be removed is not accessible to the bundle state.
    pub fn size_hint(&self) -> usize {
        self.state_size + self.reverts_size + self.contracts.len()
    }

    /// Return reference to the state.
    pub const fn state(&self) -> &HashMap<Address, ScrollBundleAccount> {
        &self.state
    }

    /// Is bundle state empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return number of changed accounts.
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// Get account from state
    pub fn account(&self, address: &Address) -> Option<&ScrollBundleAccount> {
        self.state.get(address)
    }

    /// Get bytecode from state
    pub fn bytecode(&self, hash: &B256) -> Option<Bytecode> {
        self.contracts.get(hash).cloned()
    }

    /// Consume the bundle state and return plain state.
    pub fn into_plain_state(self, is_value_known: OriginalValuesKnown) -> ScrollStateChangeset {
        // pessimistically pre-allocate assuming _all_ accounts changed.
        let state_len = self.state.len();
        let mut accounts = Vec::with_capacity(state_len);
        let mut storage = Vec::with_capacity(state_len);

        for (address, account) in self.state {
            // append account info if it is changed.
            let was_destroyed = account.was_destroyed();
            if is_value_known.is_not_known() || account.is_info_changed() {
                let info = account.info.map(ScrollAccountInfo::without_code);
                accounts.push((address, info));
            }

            // append storage changes

            // NOTE: Assumption is that revert is going to remove whole plain storage from
            // database so we can check if plain state was wiped or not.
            let mut account_storage_changed = Vec::with_capacity(account.storage.len());

            for (key, slot) in account.storage {
                // If storage was destroyed that means that storage was wiped.
                // In that case we need to check if present storage value is different then ZERO.
                let destroyed_and_not_zero = was_destroyed && !slot.present_value.is_zero();

                // If account is not destroyed check if original values was changed,
                // so we can update it.
                let not_destroyed_and_changed = !was_destroyed && slot.is_changed();

                if is_value_known.is_not_known() ||
                    destroyed_and_not_zero ||
                    not_destroyed_and_changed
                {
                    account_storage_changed.push((key, slot.present_value));
                }
            }

            if !account_storage_changed.is_empty() || was_destroyed {
                // append storage changes to account.
                storage.push(PlainStorageChangeset {
                    address,
                    wipe_storage: was_destroyed,
                    storage: account_storage_changed,
                });
            }
        }
        let contracts = self
            .contracts
            .into_iter()
            // remove empty bytecodes
            .filter(|(b, _)| *b != KECCAK_EMPTY)
            .collect::<Vec<_>>();
        ScrollStateChangeset { accounts, storage, contracts }
    }

    /// Consume the bundle state and split it into reverts and plain state.
    pub fn into_plain_state_and_reverts(
        mut self,
        is_value_known: OriginalValuesKnown,
    ) -> (ScrollStateChangeset, ScrollPlainStateReverts) {
        let reverts = self.take_all_reverts();
        let plain_state = self.into_plain_state(is_value_known);
        (plain_state, reverts.into_plain_state_reverts())
    }

    /// Return and clear all reverts from [`ScrollBundleState`]
    pub fn take_all_reverts(&mut self) -> ScrollReverts {
        self.reverts_size = 0;
        core::mem::take(&mut self.reverts)
    }
}
