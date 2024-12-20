use super::{ParallelStateRoot, PoseidonKeyHasher, StateRoot, StorageRoot};
use reth_db::transaction::DbTx;
use reth_storage_api::{BlockReader, DatabaseProviderFactory, StateCommitmentProvider};
use reth_trie_db::{DatabaseHashedCursorFactory, DatabaseTrieCursorFactory, StateCommitment};
use reth_trie_parallel::ParallelStateCommitment;

/// The state commitment type for Scroll's binary Merkle Patricia Trie.
#[derive(Debug)]
#[non_exhaustive]
pub struct BinaryMerklePatriciaTrie;

impl StateCommitment for BinaryMerklePatriciaTrie {
    type StateRoot<'a, TX: DbTx + 'a> =
        StateRoot<DatabaseTrieCursorFactory<'a, TX>, DatabaseHashedCursorFactory<'a, TX>>;
    type StorageRoot<'a, TX: DbTx + 'a> =
        StorageRoot<DatabaseTrieCursorFactory<'a, TX>, DatabaseHashedCursorFactory<'a, TX>>;
    // TODO(scroll): replace with scroll proof type
    type StateProof<'a, TX: DbTx + 'a> = reth_trie::proof::Proof<
        DatabaseTrieCursorFactory<'a, TX>,
        DatabaseHashedCursorFactory<'a, TX>,
    >;
    // TODO(scroll): replace with scroll witness type
    type StateWitness<'a, TX: DbTx + 'a> = reth_trie::witness::TrieWitness<
        DatabaseTrieCursorFactory<'a, TX>,
        DatabaseHashedCursorFactory<'a, TX>,
    >;
    type KeyHasher = PoseidonKeyHasher;
}

impl ParallelStateCommitment for BinaryMerklePatriciaTrie {
    type ParallelStateRoot<
        P: DatabaseProviderFactory<Provider: BlockReader> + StateCommitmentProvider + Clone + 'static,
    > = ParallelStateRoot<P>;
}
