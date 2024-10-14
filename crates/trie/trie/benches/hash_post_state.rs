#![allow(missing_docs, unreachable_pub)]
use alloy_primitives::{map::HashMap, Address, B256, U256};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner};
use reth_trie::{HashedPostState, HashedStorage, KeccakKeyHasher, KeyHasher};
use revm::db::{states::BundleBuilder, BundleAccount};

pub fn hash_post_state<KH: KeyHasher>(c: &mut Criterion) {
    let mut group = c.benchmark_group("Hash Post State");
    group.sample_size(20);

    for size in [100, 1_000, 3_000, 5_000, 10_000] {
        let state = generate_test_data(size);

        // sequence
        group.bench_function(BenchmarkId::new("sequence hashing", size), |b| {
            b.iter(|| from_bundle_state_seq::<KH>(&state))
        });

        // parallel
        group.bench_function(BenchmarkId::new("parallel hashing", size), |b| {
            b.iter(|| HashedPostState::from_bundle_state::<KH>(&state))
        });
    }
}

fn from_bundle_state_seq<KH: KeyHasher>(
    state: &HashMap<Address, BundleAccount>,
) -> HashedPostState {
    let mut this = HashedPostState::default();

    for (address, account) in state {
        let hashed_address = KH::hash_key(address);
        this.accounts.insert(hashed_address, account.info.clone().map(Into::into));

        let hashed_storage = HashedStorage::from_iter(
            account.status.was_destroyed(),
            account.storage.iter().map(|(key, value)| {
                (KH::hash_key(B256::new(key.to_be_bytes())), value.present_value)
            }),
        );
        this.storages.insert(hashed_address, hashed_storage);
    }
    this
}

fn generate_test_data(size: usize) -> HashMap<Address, BundleAccount> {
    let storage_size = 1_000;
    let mut runner = TestRunner::new(ProptestConfig::default());

    use proptest::collection::hash_map;
    let state = hash_map(
        any::<Address>(),
        hash_map(
            any::<U256>(), // slot
            (
                any::<U256>(), // old value
                any::<U256>(), // new value
            ),
            storage_size,
        ),
        size,
    )
    .new_tree(&mut runner)
    .unwrap()
    .current();

    let mut bundle_builder = BundleBuilder::default();

    for (address, storage) in state {
        bundle_builder = bundle_builder.state_storage(address, storage.into_iter().collect());
    }

    let bundle_state = bundle_builder.build();

    bundle_state.state
}

criterion_group!(post_state, hash_post_state::<KeccakKeyHasher>);
criterion_main!(post_state);
