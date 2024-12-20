use crate::{db::ParallelDatabaseStateRoot, root::ParallelStateRoot};
use reth_provider::{BlockReader, DatabaseProviderFactory, StateCommitmentProvider};
use reth_trie_db::MerklePatriciaTrie;

/// The `ParallelStateCommitment` trait provides associated types for parallel state commitment
/// operations.
pub trait ParallelStateCommitment {
    /// The parallel state root type.
    type ParallelStateRoot<
        P: DatabaseProviderFactory<Provider: BlockReader> + StateCommitmentProvider + Clone + 'static,
    >: ParallelDatabaseStateRoot<P>;
}

impl ParallelStateCommitment for MerklePatriciaTrie {
    type ParallelStateRoot<
        P: DatabaseProviderFactory<Provider: BlockReader> + StateCommitmentProvider + Clone + 'static,
    > = ParallelStateRoot<P>;
}
