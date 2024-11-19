use super::{PoseidonKeyHasher, PosiedonValueHasher};
use alloy_primitives::{Address, BlockNumber, B256};
use reth_db::transaction::DbTx;
use reth_execution_errors::{StateRootError, StorageRootError};
use reth_primitives::constants::EMPTY_ROOT_HASH;
use reth_trie::{
    hashed_cursor::{HashedCursorFactory, HashedPostStateCursorFactory, HashedStorageCursor},
    key::BitsCompatibility,
    node_iter::{TrieElement, TrieNodeIter},
    prefix_set::{PrefixSet, TriePrefixSets},
    stats::TrieTracker,
    trie_cursor::{InMemoryTrieCursorFactory, TrieCursorFactory},
    updates::{StorageTrieUpdates, TrieUpdates},
    walker::TrieWalker,
    HashedPostState, IntermediateStateRootState, Nibbles, StateRootProgress, TrieAccount,
    TrieInput,
};
use scroll_primitives::ScrollTrieAccount;
use scroll_trie::HashBuilder;
use tracing::{debug, trace};

#[cfg(feature = "metrics")]
use crate::metrics::{StateRootMetrics, TrieRootMetrics};

// TODO(frisitano): Instead of introducing this new type we should

/// `StateRoot` is used to compute the root node of a state trie.
#[derive(Debug)]
pub struct StateRoot<T, H> {
    /// The factory for trie cursors.
    pub trie_cursor_factory: T,
    /// The factory for hashed cursors.
    pub hashed_cursor_factory: H,
    /// A set of prefix sets that have changed.
    pub prefix_sets: TriePrefixSets,
    /// Previous intermediate state.
    previous_state: Option<IntermediateStateRootState>,
    /// The number of updates after which the intermediate progress should be returned.
    threshold: u64,
    #[cfg(feature = "metrics")]
    /// State root metrics.
    metrics: StateRootMetrics,
}

impl<T, H> StateRoot<T, H> {
    /// Creates [`StateRoot`] with `trie_cursor_factory` and `hashed_cursor_factory`. All other
    /// parameters are set to reasonable defaults.
    ///
    /// The cursors created by given factories are then used to walk through the accounts and
    /// calculate the state root value with.
    pub fn new(trie_cursor_factory: T, hashed_cursor_factory: H) -> Self {
        Self {
            trie_cursor_factory,
            hashed_cursor_factory,
            prefix_sets: TriePrefixSets::default(),
            previous_state: None,
            threshold: 100_000,
            #[cfg(feature = "metrics")]
            metrics: StateRootMetrics::default(),
        }
    }

    /// Set the prefix sets.
    pub fn with_prefix_sets(mut self, prefix_sets: TriePrefixSets) -> Self {
        self.prefix_sets = prefix_sets;
        self
    }

    /// Set the threshold.
    pub const fn with_threshold(mut self, threshold: u64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the threshold to maximum value so that intermediate progress is not returned.
    pub const fn with_no_threshold(mut self) -> Self {
        self.threshold = u64::MAX;
        self
    }

    /// Set the previously recorded intermediate state.
    pub fn with_intermediate_state(mut self, state: Option<IntermediateStateRootState>) -> Self {
        self.previous_state = state;
        self
    }

    /// Set the hashed cursor factory.
    pub fn with_hashed_cursor_factory<HF>(self, hashed_cursor_factory: HF) -> StateRoot<T, HF> {
        StateRoot {
            trie_cursor_factory: self.trie_cursor_factory,
            hashed_cursor_factory,
            prefix_sets: self.prefix_sets,
            threshold: self.threshold,
            previous_state: self.previous_state,
            #[cfg(feature = "metrics")]
            metrics: self.metrics,
        }
    }

    /// Set the trie cursor factory.
    pub fn with_trie_cursor_factory<TF>(self, trie_cursor_factory: TF) -> StateRoot<TF, H> {
        StateRoot {
            trie_cursor_factory,
            hashed_cursor_factory: self.hashed_cursor_factory,
            prefix_sets: self.prefix_sets,
            threshold: self.threshold,
            previous_state: self.previous_state,
            #[cfg(feature = "metrics")]
            metrics: self.metrics,
        }
    }
}

impl<T, H> StateRoot<T, H>
where
    T: TrieCursorFactory + Clone,
    H: HashedCursorFactory + Clone,
{
    /// Walks the intermediate nodes of existing state trie (if any) and hashed entries. Feeds the
    /// nodes into the hash builder. Collects the updates in the process.
    ///
    /// Ignores the threshold.
    ///
    /// # Returns
    ///
    /// The intermediate progress of state root computation and the trie updates.
    pub fn root_with_updates(self) -> Result<(B256, TrieUpdates), StateRootError> {
        match self.with_no_threshold().calculate(true)? {
            StateRootProgress::Complete(root, _, updates) => Ok((root, updates)),
            StateRootProgress::Progress(..) => unreachable!(), // unreachable threshold
        }
    }

    /// Walks the intermediate nodes of existing state trie (if any) and hashed entries. Feeds the
    /// nodes into the hash builder.
    ///
    /// # Returns
    ///
    /// The state root hash.
    pub fn root(self) -> Result<B256, StateRootError> {
        match self.calculate(false)? {
            StateRootProgress::Complete(root, _, _) => Ok(root),
            StateRootProgress::Progress(..) => unreachable!(), // update retenion is disabled
        }
    }

    /// Walks the intermediate nodes of existing state trie (if any) and hashed entries. Feeds the
    /// nodes into the hash builder. Collects the updates in the process.
    ///
    /// # Returns
    ///
    /// The intermediate progress of state root computation.
    pub fn root_with_progress(self) -> Result<StateRootProgress, StateRootError> {
        self.calculate(true)
    }

    fn calculate(self, retain_updates: bool) -> Result<StateRootProgress, StateRootError> {
        trace!(target: "trie::state_root", "calculating state root");
        let mut tracker = TrieTracker::default();
        let mut trie_updates = TrieUpdates::default();

        let trie_cursor = self.trie_cursor_factory.account_trie_cursor()?;

        let hashed_account_cursor = self.hashed_cursor_factory.hashed_account_cursor()?;
        let (mut hash_builder, mut account_node_iter) = match self.previous_state {
            Some(state) => {
                let hash_builder = state.hash_builder.with_updates(retain_updates).into();

                let walker = TrieWalker::from_stack(
                    trie_cursor,
                    state.walker_stack,
                    self.prefix_sets.account_prefix_set,
                )
                .with_deletions_retained(retain_updates);
                let node_iter = TrieNodeIter::new(walker, hashed_account_cursor)
                    .with_last_hashed_key(state.last_account_key);
                (hash_builder, node_iter)
            }
            None => {
                let hash_builder = HashBuilder::default().with_updates(retain_updates);
                let walker = TrieWalker::new(trie_cursor, self.prefix_sets.account_prefix_set)
                    .with_deletions_retained(retain_updates);
                let node_iter = TrieNodeIter::new(walker, hashed_account_cursor);
                (hash_builder, node_iter)
            }
        };

        let mut hashed_entries_walked = 0;
        let mut updated_storage_nodes = 0;
        while let Some(node) = account_node_iter.try_next()? {
            match node {
                TrieElement::Branch(node) => {
                    tracker.inc_branch();
                    hash_builder.add_branch(node.key, node.value, node.children_are_in_trie);
                }
                TrieElement::Leaf(hashed_address, account) => {
                    tracker.inc_leaf();
                    hashed_entries_walked += 1;

                    // We assume we can always calculate a storage root without
                    // OOMing. This opens us up to a potential DOS vector if
                    // a contract had too many storage entries and they were
                    // all buffered w/o us returning and committing our intermediate
                    // progress.
                    // TODO: We can consider introducing the TrieProgress::Progress/Complete
                    // abstraction inside StorageRoot, but let's give it a try as-is for now.
                    let storage_root_calculator = StorageRoot::new_hashed(
                        self.trie_cursor_factory.clone(),
                        self.hashed_cursor_factory.clone(),
                        hashed_address,
                        #[cfg(feature = "metrics")]
                        self.metrics.storage_trie.clone(),
                    )
                    .with_prefix_set(
                        self.prefix_sets
                            .storage_prefix_sets
                            .get(&hashed_address)
                            .cloned()
                            .unwrap_or_default(),
                    );

                    let storage_root = if retain_updates {
                        let (root, storage_slots_walked, updates) =
                            storage_root_calculator.root_with_updates()?;
                        hashed_entries_walked += storage_slots_walked;
                        // We only walk over hashed address once, so it's safe to insert.
                        updated_storage_nodes += updates.len();
                        trie_updates.insert_storage_updates(hashed_address, updates);
                        root
                    } else {
                        storage_root_calculator.root()?
                    };

                    let account: ScrollTrieAccount =
                        TrieAccount::from((account, storage_root)).into();
                    let account_hash = PosiedonValueHasher::hash_account(account);
                    hash_builder.add_leaf(
                        Nibbles::unpack_and_truncate_bits(hashed_address),
                        account_hash.as_slice(),
                    );

                    // Decide if we need to return intermediate progress.
                    let total_updates_len = updated_storage_nodes +
                        account_node_iter.walker.removed_keys_len() +
                        hash_builder.updates_len();
                    if retain_updates && total_updates_len as u64 >= self.threshold {
                        let (walker_stack, walker_deleted_keys) = account_node_iter.walker.split();
                        trie_updates.removed_nodes.extend(walker_deleted_keys);
                        let (hash_builder, hash_builder_updates) = hash_builder.split();
                        trie_updates.account_nodes.extend(hash_builder_updates);

                        let state = IntermediateStateRootState {
                            hash_builder: hash_builder.into(),
                            walker_stack,
                            last_account_key: hashed_address,
                        };

                        return Ok(StateRootProgress::Progress(
                            Box::new(state),
                            hashed_entries_walked,
                            trie_updates,
                        ))
                    }
                }
            }
        }

        let root = hash_builder.root();

        trie_updates.finalize(
            account_node_iter.walker,
            hash_builder.into(),
            self.prefix_sets.destroyed_accounts,
        );

        let stats = tracker.finish();

        #[cfg(feature = "metrics")]
        self.metrics.state_trie.record(stats);

        trace!(
            target: "trie::state_root",
            %root,
            duration = ?stats.duration(),
            branches_added = stats.branches_added(),
            leaves_added = stats.leaves_added(),
            "calculated state root"
        );

        Ok(StateRootProgress::Complete(root, hashed_entries_walked, trie_updates))
    }
}

/// `StorageRoot` is used to compute the root node of an account storage trie.
#[derive(Debug)]
pub struct StorageRoot<T, H> {
    /// A reference to the database transaction.
    pub trie_cursor_factory: T,
    /// The factory for hashed cursors.
    pub hashed_cursor_factory: H,
    /// The hashed address of an account.
    pub hashed_address: B256,
    /// The set of storage slot prefixes that have changed.
    pub prefix_set: PrefixSet,
    /// Storage root metrics.
    #[cfg(feature = "metrics")]
    metrics: TrieRootMetrics,
}

impl<T, H> StorageRoot<T, H> {
    /// Creates a new storage root calculator given a raw address.
    pub fn new(
        trie_cursor_factory: T,
        hashed_cursor_factory: H,
        address: Address,
        #[cfg(feature = "metrics")] metrics: TrieRootMetrics,
    ) -> Self {
        Self::new_hashed(
            trie_cursor_factory,
            hashed_cursor_factory,
            PoseidonKeyHasher::hash_key(address),
            #[cfg(feature = "metrics")]
            metrics,
        )
    }

    /// Creates a new storage root calculator given a hashed address.
    pub fn new_hashed(
        trie_cursor_factory: T,
        hashed_cursor_factory: H,
        hashed_address: B256,
        #[cfg(feature = "metrics")] metrics: TrieRootMetrics,
    ) -> Self {
        Self {
            trie_cursor_factory,
            hashed_cursor_factory,
            hashed_address,
            prefix_set: PrefixSet::default(),
            #[cfg(feature = "metrics")]
            metrics,
        }
    }

    /// Set the changed prefixes.
    pub fn with_prefix_set(mut self, prefix_set: PrefixSet) -> Self {
        self.prefix_set = prefix_set;
        self
    }

    /// Set the hashed cursor factory.
    pub fn with_hashed_cursor_factory<HF>(self, hashed_cursor_factory: HF) -> StorageRoot<T, HF> {
        StorageRoot {
            trie_cursor_factory: self.trie_cursor_factory,
            hashed_cursor_factory,
            hashed_address: self.hashed_address,
            prefix_set: self.prefix_set,
            #[cfg(feature = "metrics")]
            metrics: self.metrics,
        }
    }

    /// Set the trie cursor factory.
    pub fn with_trie_cursor_factory<TF>(self, trie_cursor_factory: TF) -> StorageRoot<TF, H> {
        StorageRoot {
            trie_cursor_factory,
            hashed_cursor_factory: self.hashed_cursor_factory,
            hashed_address: self.hashed_address,
            prefix_set: self.prefix_set,
            #[cfg(feature = "metrics")]
            metrics: self.metrics,
        }
    }
}

impl<T, H> StorageRoot<T, H>
where
    T: TrieCursorFactory,
    H: HashedCursorFactory,
{
    /// Walks the hashed storage table entries for a given address and calculates the storage root.
    ///
    /// # Returns
    ///
    /// The storage root and storage trie updates for a given address.
    pub fn root_with_updates(self) -> Result<(B256, usize, StorageTrieUpdates), StorageRootError> {
        self.calculate(true)
    }

    /// Walks the hashed storage table entries for a given address and calculates the storage root.
    ///
    /// # Returns
    ///
    /// The storage root.
    pub fn root(self) -> Result<B256, StorageRootError> {
        let (root, _, _) = self.calculate(false)?;
        Ok(root)
    }

    /// Walks the hashed storage table entries for a given address and calculates the storage root.
    ///
    /// # Returns
    ///
    /// The storage root, number of walked entries and trie updates
    /// for a given address if requested.
    pub fn calculate(
        self,
        retain_updates: bool,
    ) -> Result<(B256, usize, StorageTrieUpdates), StorageRootError> {
        trace!(target: "trie::storage_root", hashed_address = ?self.hashed_address, "calculating storage root");

        let mut hashed_storage_cursor =
            self.hashed_cursor_factory.hashed_storage_cursor(self.hashed_address)?;

        // short circuit on empty storage
        if hashed_storage_cursor.is_storage_empty()? {
            return Ok((EMPTY_ROOT_HASH, 0, StorageTrieUpdates::deleted()))
        }

        let mut tracker = TrieTracker::default();
        let trie_cursor = self.trie_cursor_factory.storage_trie_cursor(self.hashed_address)?;
        let walker =
            TrieWalker::new(trie_cursor, self.prefix_set).with_deletions_retained(retain_updates);

        let mut hash_builder = HashBuilder::default().with_updates(retain_updates);

        let mut storage_node_iter = TrieNodeIter::new(walker, hashed_storage_cursor);
        while let Some(node) = storage_node_iter.try_next()? {
            match node {
                TrieElement::Branch(node) => {
                    tracker.inc_branch();
                    hash_builder.add_branch(node.key, node.value, node.children_are_in_trie);
                }
                TrieElement::Leaf(hashed_slot, value) => {
                    let hashed_value = PosiedonValueHasher::hash_storage(value);
                    tracker.inc_leaf();
                    hash_builder.add_leaf(
                        Nibbles::unpack_and_truncate_bits(hashed_slot),
                        hashed_value.as_ref(),
                    );
                }
            }
        }

        let root = hash_builder.root();

        let mut trie_updates = StorageTrieUpdates::default();
        trie_updates.finalize(storage_node_iter.walker, hash_builder.into());

        let stats = tracker.finish();

        #[cfg(feature = "metrics")]
        self.metrics.record(stats);

        trace!(
            target: "trie::storage_root",
            %root,
            hashed_address = %self.hashed_address,
            duration = ?stats.duration(),
            branches_added = stats.branches_added(),
            leaves_added = stats.leaves_added(),
            "calculated storage root"
        );

        let storage_slots_walked = stats.leaves_added() as usize;
        Ok((root, storage_slots_walked, trie_updates))
    }
}

use reth_trie_db::{
    DatabaseHashedCursorFactory, DatabaseStateRoot, DatabaseTrieCursorFactory, PrefixSetLoader,
};
use std::ops::RangeInclusive;

impl<'a, TX: DbTx> DatabaseStateRoot<'a, TX>
    for StateRoot<DatabaseTrieCursorFactory<'a, TX>, DatabaseHashedCursorFactory<'a, TX>>
{
    fn from_tx(tx: &'a TX) -> Self {
        Self::new(DatabaseTrieCursorFactory::new(tx), DatabaseHashedCursorFactory::new(tx))
    }

    fn incremental_root_calculator(
        tx: &'a TX,
        range: RangeInclusive<BlockNumber>,
    ) -> Result<Self, StateRootError> {
        let loaded_prefix_sets = PrefixSetLoader::new(tx).load(range)?;
        Ok(Self::from_tx(tx).with_prefix_sets(loaded_prefix_sets))
    }

    fn incremental_root(
        tx: &'a TX,
        range: RangeInclusive<BlockNumber>,
    ) -> Result<B256, StateRootError> {
        debug!(target: "trie::loader", ?range, "incremental state root");
        Self::incremental_root_calculator(tx, range)?.root()
    }

    fn incremental_root_with_updates(
        tx: &'a TX,
        range: RangeInclusive<BlockNumber>,
    ) -> Result<(B256, TrieUpdates), StateRootError> {
        debug!(target: "trie::loader", ?range, "incremental state root");
        Self::incremental_root_calculator(tx, range)?.root_with_updates()
    }

    fn incremental_root_with_progress(
        tx: &'a TX,
        range: RangeInclusive<BlockNumber>,
    ) -> Result<StateRootProgress, StateRootError> {
        debug!(target: "trie::loader", ?range, "incremental state root with progress");
        Self::incremental_root_calculator(tx, range)?.root_with_progress()
    }

    fn overlay_root(tx: &'a TX, post_state: HashedPostState) -> Result<B256, StateRootError> {
        let prefix_sets = post_state.construct_prefix_sets().freeze();
        let state_sorted = post_state.into_sorted();
        StateRoot::new(
            DatabaseTrieCursorFactory::new(tx),
            HashedPostStateCursorFactory::new(DatabaseHashedCursorFactory::new(tx), &state_sorted),
        )
        .with_prefix_sets(prefix_sets)
        .root()
    }

    fn overlay_root_with_updates(
        tx: &'a TX,
        post_state: HashedPostState,
    ) -> Result<(B256, TrieUpdates), StateRootError> {
        let prefix_sets = post_state.construct_prefix_sets().freeze();
        let state_sorted = post_state.into_sorted();
        StateRoot::new(
            DatabaseTrieCursorFactory::new(tx),
            HashedPostStateCursorFactory::new(DatabaseHashedCursorFactory::new(tx), &state_sorted),
        )
        .with_prefix_sets(prefix_sets)
        .root_with_updates()
    }

    fn overlay_root_from_nodes(tx: &'a TX, input: TrieInput) -> Result<B256, StateRootError> {
        let state_sorted = input.state.into_sorted();
        let nodes_sorted = input.nodes.into_sorted();
        StateRoot::new(
            InMemoryTrieCursorFactory::new(DatabaseTrieCursorFactory::new(tx), &nodes_sorted),
            HashedPostStateCursorFactory::new(DatabaseHashedCursorFactory::new(tx), &state_sorted),
        )
        .with_prefix_sets(input.prefix_sets.freeze())
        .root()
    }

    fn overlay_root_from_nodes_with_updates(
        tx: &'a TX,
        input: TrieInput,
    ) -> Result<(B256, TrieUpdates), StateRootError> {
        let state_sorted = input.state.into_sorted();
        let nodes_sorted = input.nodes.into_sorted();
        StateRoot::new(
            InMemoryTrieCursorFactory::new(DatabaseTrieCursorFactory::new(tx), &nodes_sorted),
            HashedPostStateCursorFactory::new(DatabaseHashedCursorFactory::new(tx), &state_sorted),
        )
        .with_prefix_sets(input.prefix_sets.freeze())
        .root_with_updates()
    }
}

#[cfg(test)]
mod test {
    use super::StateRoot;
    use alloy_consensus::constants::KECCAK_EMPTY;
    use alloy_primitives::{
        aliases::U248, b256, hex_literal::hex, keccak256, Address, FixedBytes, Uint, B256, U256,
    };
    use proptest::{prelude::ProptestConfig, proptest};
    use proptest_arbitrary_interop::arb;
    use reth_db::{
        cursor::{DbCursorRO, DbCursorRW, DbDupCursorRO},
        tables,
        test_utils::TempDatabase,
        transaction::{DbTx, DbTxMut},
        DatabaseEnv,
    };
    use reth_primitives::{constants::EMPTY_ROOT_HASH, Account, StorageEntry};
    use reth_provider::{
        test_utils::create_test_provider_factory, DatabaseProviderRW, StorageTrieWriter, TrieWriter,
    };
    use reth_trie::{
        prefix_set::PrefixSetMut,
        test_utils::{state_root, state_root_prehashed, storage_root, storage_root_prehashed},
        BranchNodeCompact, StorageRoot, TrieMask,
    };
    use reth_trie_db::{DatabaseStateRoot, DatabaseStorageRoot, DatabaseTrieCursorFactory};
    use std::{
        collections::{BTreeMap, HashMap},
        ops::Mul,
        str::FromStr,
        sync::Arc,
    };

    use alloy_rlp::Encodable;
    use poseidon_bn254::{hash_with_domain, Fr, PrimeField};
    use reth_trie::{
        prefix_set::TriePrefixSets, trie_cursor::InMemoryTrieCursorFactory,
        updates::StorageTrieUpdates, HashedPostState, IntermediateStateRootState, Nibbles,
        StateRootProgress, TrieAccount,
    };
    use std::sync::Once;
    use tracing_subscriber::{self, fmt::format::FmtSpan};
    use zktrie::HashField;
    use zktrie_rust::{
        db::SimpleDb,
        hash::AsHash,
        types::{Hashable, TrieHashScheme},
    };

    static INIT: Once = Once::new();

    pub fn init_test_logger() {
        INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_test_writer() // Capture logs for test output
                .with_span_events(FmtSpan::CLOSE) // Optional: Add span events
                .with_env_filter("trace") // Set log level as needed
                .init();
        });
    }

    // TODO(frisitano): Clean up tests and add tests for storage trie.

    type State = BTreeMap<Address, (Account, BTreeMap<B256, U256>)>;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1, ..ProptestConfig::default()
        })]

        #[test]
        fn fuzz_in_memory_account_nodes(mut init_state: BTreeMap<B256, (u32, U256, Option<B256>)>, state_updates: [BTreeMap<B256, Option<U256>>; 4]) {
            // init_test_logger();

            let mut init_state: BTreeMap<B256, Account> = init_state.into_iter().take(1).map(|(mut address, (nonce, mut balance, bytecode_hash))| {
                // set the largest byte to 0
                <B256 as AsMut<[u8; 32]>>::as_mut(&mut address)[31] = 0;
                // set the most significant 8 bits to 0
                unsafe {
                    balance.as_limbs_mut()[3] &= 0x00FFFFFFFFFFFFFF;
                }
                let account = Account { balance, nonce: nonce.into(), bytecode_hash };
                (address, account)
            }).collect();
            let state_updates: Vec<BTreeMap<_, _>> = state_updates.into_iter().map(|update| {
                let update = update.into_iter().take(1).map(|(mut address, mut update)| {
                    // set the largest byte to 0
                    <B256 as AsMut<[u8; 32]>>::as_mut(&mut address)[31] = 0;
                    // set the most significant 8 bits to 0
                    let account = if let Some(mut balance) = update {
                        unsafe {
                            balance.as_limbs_mut()[3] &= 0x00FFFFFFFFFFFFFF;
                        }
                        Some(Account { balance, ..Default::default() })
                    } else { None };
                    (address, account)
                }).collect::<BTreeMap<_, _>>();
                update
            }).collect();


            let factory = create_test_provider_factory();
            let provider = factory.provider_rw().unwrap();
            let mut hashed_account_cursor = provider.tx_ref().cursor_write::<tables::HashedAccounts>().unwrap();

            // Insert init state into database
            for (hashed_address, account) in init_state.clone().into_iter() {
                hashed_account_cursor.upsert(reverse_bits(hashed_address), account).unwrap();
            }

            // Compute initial root and updates
            let (_, mut trie_nodes) = StateRoot::from_tx(provider.tx_ref())
                .root_with_updates()
                .unwrap();

            let mut state = init_state;
            for state_update in state_updates {
                // Insert state updates into database
                let mut hashed_state = HashedPostState::default();
                for (hashed_address, account) in state_update.into_iter().take(4) {
                    if let Some(account) = account {
                        hashed_account_cursor.upsert(reverse_bits(hashed_address), account).unwrap();
                        hashed_state.accounts.insert(reverse_bits(hashed_address), Some(account));
                        state.insert(hashed_address, account);
                    } else {
                        hashed_state.accounts.insert(reverse_bits(hashed_address), None);
                        state.remove(&hashed_address);
                    }
                }

                // Compute root with in-memory trie nodes overlay
                let (state_root, trie_updates) = StateRoot::from_tx(provider.tx_ref())
                    .with_prefix_sets(hashed_state.construct_prefix_sets().freeze())
                    .with_trie_cursor_factory(InMemoryTrieCursorFactory::new(
                        DatabaseTrieCursorFactory::new(provider.tx_ref()), &trie_nodes.clone().into_sorted())
                    )
                    .root_with_updates()
                    .unwrap();

                trie_nodes.extend(trie_updates);

                // Verify the result
                let expected_root = state_root_zktrie(
                    state.iter().map(|(key, account)| (*key, (*account, std::iter::empty())))
                );
                assert_eq!(expected_root.0, state_root.0);

            }
        }


    }

    fn reverse_bits(b256: B256) -> B256 {
        let mut b256 = b256.0;
        for byte in b256.iter_mut() {
            *byte = byte.reverse_bits();
        }
        B256::from(b256)
    }

    fn state_root_zktrie<I, S>(accounts: I) -> B256
    where
        I: IntoIterator<Item = (B256, (Account, S))>,
        S: IntoIterator<Item = (B256, U256)>,
    {
        let mut trie = zktrie();
        const COMPRESSION_FLAG: u32 = 8;
        for (address, (account, storage)) in accounts.into_iter() {
            let mut key = address.0;
            key.reverse();
            let key = AsHash::from_bytes(&key).unwrap();
            let mut account_bytes = Vec::with_capacity(5);

            account_bytes.push(U256::from_limbs([account.nonce, 0, 0, 0]).to_be_bytes());
            account_bytes.push(account.balance.to_be_bytes());
            account_bytes.push([0u8; 32]);
            account_bytes.push(account.bytecode_hash.unwrap_or(KECCAK_EMPTY).0);
            account_bytes.push([0u8; 32]);

            // for bytes in account_bytes.iter() {
            //     println!("{:?}", bytes);
            // }

            trie.try_update(&key, COMPRESSION_FLAG, account_bytes).unwrap();
        }
        trie.prepare_root().unwrap();
        let mut root = trie.root().to_bytes();
        root.reverse();
        B256::from_slice(&root)
        // 00
    }

    #[test]
    fn test_basic_state_root_with_updates_succeeds() {
        let address_1 = Address::with_last_byte(0);
        let address_2 = Address::with_last_byte(3);
        let address_3 = Address::with_last_byte(7);
        let account_1 = Account { balance: Uint::from(1), ..Default::default() };
        let account_2 = Account { balance: Uint::from(2), ..Default::default() };
        let account_3 = Account { balance: Uint::from(3), ..Default::default() };

        let factory = create_test_provider_factory();
        let tx = factory.provider_rw().unwrap();

        insert_account(tx.tx_ref(), address_1, account_1, &Default::default());
        insert_account(tx.tx_ref(), address_2, account_2, &Default::default());
        insert_account(tx.tx_ref(), address_3, account_3, &Default::default());

        tx.commit().unwrap();

        let tx = factory.provider_rw().unwrap();
        let (root, updates) = StateRoot::from_tx(tx.tx_ref()).root_with_updates().unwrap();
    }

    fn test_state_root_with_state(state: State) {
        let factory = create_test_provider_factory();
        let tx = factory.provider_rw().unwrap();

        for (address, (account, storage)) in &state {
            insert_account(tx.tx_ref(), *address, *account, storage)
        }
        tx.commit().unwrap();
        let expected = state_root(state);

        let tx = factory.provider_rw().unwrap();
        let got = StateRoot::from_tx(tx.tx_ref()).root().unwrap();
        assert_eq!(expected, got);
    }

    fn insert_account(
        tx: &impl DbTxMut,
        address: Address,
        account: Account,
        storage: &BTreeMap<B256, U256>,
    ) {
        let hashed_address = keccak256(address);
        tx.put::<tables::HashedAccounts>(hashed_address, account).unwrap();
        insert_storage(tx, hashed_address, storage);
    }

    fn insert_storage(tx: &impl DbTxMut, hashed_address: B256, storage: &BTreeMap<B256, U256>) {
        for (k, v) in storage {
            tx.put::<tables::HashedStorages>(
                hashed_address,
                StorageEntry { key: keccak256(k), value: *v },
            )
            .unwrap();
        }
    }

    fn poseidon_hash_scheme(a: &[u8; 32], b: &[u8; 32], domain: &[u8; 32]) -> Option<[u8; 32]> {
        let a = Fr::from_repr_vartime(*a)?;
        let b = Fr::from_repr_vartime(*b)?;
        let domain = Fr::from_repr_vartime(*domain)?;
        Some(hash_with_domain(&[a, b], domain).to_repr())
    }

    fn zktrie() -> zktrie_rust::raw::ZkTrieImpl<AsHash<HashField>, SimpleDb, 248> {
        zktrie::init_hash_scheme_simple(poseidon_hash_scheme);
        zktrie_rust::raw::ZkTrieImpl::<AsHash<HashField>, SimpleDb, 248>::new_zktrie_impl(
            SimpleDb::new(),
        )
        .unwrap()
    }
}
