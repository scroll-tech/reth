use alloy_primitives::B256;
use reth_execution_errors::ParallelStateRootError;
use reth_provider::providers::ConsistentDbView;
use reth_storage_errors::provider::ProviderError;
use reth_trie::TrieInput;
use reth_trie_common::updates::TrieUpdates;

/// A type that derives the root in parallel on the current state in database.
/// The type requires to be created from a view of the database which will remain unchanged during
/// the whole state root computation process.
pub trait ParallelDatabaseStateRoot<P>: Sized {
    /// Returns a parallel state root instance from the [`ConsistentDbView`].
    fn from_consistent_db_view(view: ConsistentDbView<P>) -> Result<Self, ProviderError>;
    /// Calculate the state root in parallel.
    fn incremental_root(self, input: TrieInput) -> Result<B256, ParallelStateRootError>;
    /// Calculate the state root in parallel and returns the trie updates.
    fn incremental_root_with_updates(
        self,
        input: TrieInput,
    ) -> Result<(B256, TrieUpdates), ParallelStateRootError>;
}
