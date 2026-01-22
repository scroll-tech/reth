use reth_chainspec::EthChainSpec;
use reth_node_api::{FullNodeTypes, NodeTypes};
use reth_node_builder::{
    components::{PoolBuilder, PoolBuilderConfigOverrides},
    BuilderContext, TxTy,
};

use reth_provider::CanonStateSubscriptions;
use reth_scroll_chainspec::{ChainConfig, ScrollChainConfig};
use reth_scroll_evm::ScrollBaseFeeProvider;
use reth_scroll_txpool::{ScrollTransactionPool, ScrollTransactionValidator};
use reth_transaction_pool::{
    blobstore::DiskFileBlobStore, CoinbaseTipOrdering, EthPoolTransaction,
    TransactionValidationTaskExecutor,
};
use scroll_alloy_consensus::ScrollTransaction;
use scroll_alloy_hardforks::ScrollHardforks;

/// A basic scroll transaction pool.
///
/// This contains various settings that can be configured and take precedence over the node's
/// config.
#[derive(Debug, Clone)]
pub struct ScrollPoolBuilder<T = reth_scroll_txpool::ScrollPooledTransaction> {
    /// Enforced overrides that are applied to the pool config.
    pub pool_config_overrides: PoolBuilderConfigOverrides,
    /// Require L1 data fee buffer in balance check.
    /// When enabled, validates balance >= `L2_cost` + 2*`L1_cost` but only charges `L2_cost` +
    /// 1*`L1_cost`.
    pub require_l1_data_fee_buffer: bool,
    /// Marker for the pooled transaction type.
    _pd: core::marker::PhantomData<T>,
}

impl<T> Default for ScrollPoolBuilder<T> {
    fn default() -> Self {
        Self {
            pool_config_overrides: Default::default(),
            require_l1_data_fee_buffer: false,
            _pd: Default::default(),
        }
    }
}

impl<T> ScrollPoolBuilder<T> {
    /// Sets the [`PoolBuilderConfigOverrides`] on the pool builder.
    pub fn with_pool_config_overrides(
        mut self,
        pool_config_overrides: PoolBuilderConfigOverrides,
    ) -> Self {
        self.pool_config_overrides = pool_config_overrides;
        self
    }

    /// Sets the require L1 data fee buffer flag.
    /// When enabled, validates balance >= `L2_cost` + 2*`L1_cost` but only charges `L2_cost` +
    /// 1*`L1_cost`. This matches geth's block validation behavior.
    pub const fn with_require_l1_data_fee_buffer(mut self, require: bool) -> Self {
        self.require_l1_data_fee_buffer = require;
        self
    }
}

impl<Node, T> PoolBuilder<Node> for ScrollPoolBuilder<T>
where
    Node: FullNodeTypes<
        Types: NodeTypes<
            ChainSpec: EthChainSpec + ScrollHardforks + ChainConfig<Config = ScrollChainConfig>,
        >,
    >,
    T: EthPoolTransaction<Consensus = TxTy<Node::Types>> + ScrollTransaction,
{
    type Pool = ScrollTransactionPool<Node::Provider, DiskFileBlobStore, T>;

    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let Self { pool_config_overrides, require_l1_data_fee_buffer, .. } = self;
        let data_dir = ctx.config().datadir();
        let blob_store = DiskFileBlobStore::open(data_dir.blobstore(), Default::default())?;

        let validator = TransactionValidationTaskExecutor::eth_builder(ctx.provider().clone())
            .no_eip4844()
            .with_head_timestamp(ctx.head().timestamp)
            .kzg_settings(ctx.kzg_settings()?)
            .with_local_transactions_config(
                pool_config_overrides.clone().apply(ctx.pool_config()).local_transactions_config,
            )
            .with_max_tx_input_bytes(ctx.chain_spec().chain_config().max_tx_payload_bytes_per_block)
            .with_additional_tasks(
                pool_config_overrides
                    .additional_validation_tasks
                    .unwrap_or_else(|| ctx.config().txpool.additional_validation_tasks),
            )
            .build_with_tasks(ctx.task_executor().clone(), blob_store.clone())
            .map(|validator| {
                ScrollTransactionValidator::new(validator)
                    // In --dev mode we can't require gas fees because we're unable to decode
                    // the L1 block info
                    .require_l1_data_gas_fee(!ctx.config().dev.dev)
                    .require_l1_data_fee_buffer(require_l1_data_fee_buffer)
            });

        let transaction_pool = reth_transaction_pool::Pool::new(
            validator,
            CoinbaseTipOrdering::default(),
            blob_store,
            pool_config_overrides.apply(ctx.pool_config()),
        );
        tracing::info!(target: "reth::cli", "Transaction pool initialized");
        let transactions_path = data_dir.txpool_transactions();

        // spawn txpool maintenance tasks
        {
            let chain_events = ctx.provider().canonical_state_stream();
            let client = ctx.provider().clone();
            let transactions_backup_config =
                reth_transaction_pool::maintain::LocalTransactionBackupConfig::with_local_txs_backup(transactions_path);
            let base_fee_provider = ScrollBaseFeeProvider::new(ctx.chain_spec());

            ctx.task_executor().spawn_critical_with_graceful_shutdown_signal(
                "local transactions backup task",
                |shutdown| {
                    reth_transaction_pool::maintain::backup_local_transactions_task(
                        shutdown,
                        transaction_pool.clone(),
                        transactions_backup_config,
                    )
                },
            );

            // spawn the main maintenance task
            ctx.task_executor().spawn_critical(
                "txpool maintenance task",
                reth_transaction_pool::maintain::maintain_transaction_pool_future(
                    client,
                    base_fee_provider,
                    transaction_pool.clone(),
                    chain_events,
                    ctx.task_executor().clone(),
                    reth_transaction_pool::maintain::MaintainPoolConfig {
                        max_tx_lifetime: transaction_pool.config().max_queued_lifetime,
                        ..Default::default()
                    },
                ),
            );
            tracing::debug!(target: "reth::cli", "Spawned txpool maintenance task");
        }

        Ok(transaction_pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrollNode;

    use alloy_consensus::{transaction::Recovered, Header, Signed, TxLegacy};
    use alloy_primitives::{private::rand::random_iter, Bytes, Sealed, Signature, B256, U256};
    use reth_chainspec::Head;
    use reth_db::mock::DatabaseMock;
    use reth_node_api::FullNodeTypesAdapter;
    use reth_node_builder::common::WithConfigs;
    use reth_node_core::node_config::NodeConfig;
    use reth_primitives_traits::{
        transaction::error::InvalidTransactionError, GotExpected, GotExpectedBoxed,
    };
    use reth_provider::{
        noop::NoopProvider,
        test_utils::{ExtendedAccount, MockEthProvider},
    };
    use reth_scroll_chainspec::{ScrollChainSpec, SCROLL_DEV, SCROLL_MAINNET};
    use reth_scroll_primitives::{ScrollBlock, ScrollPrimitives};
    use reth_scroll_txpool::ScrollPooledTransaction;
    use reth_tasks::TaskManager;
    use reth_transaction_pool::{
        blobstore::NoopBlobStore,
        error::{InvalidPoolTransactionError, PoolErrorKind},
        PoolConfig, TransactionOrigin, TransactionPool,
    };
    use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};
    use scroll_alloy_evm::gas_price_oracle::L1_GAS_PRICE_ORACLE_ADDRESS;

    async fn pool() -> (
        ScrollTransactionPool<NoopProvider<ScrollChainSpec, ScrollPrimitives>, DiskFileBlobStore>,
        TaskManager,
    ) {
        let handle = tokio::runtime::Handle::current();
        let manager = TaskManager::new(handle);
        let config = WithConfigs {
            config: NodeConfig::new(SCROLL_MAINNET.clone()),
            toml_config: Default::default(),
        };

        let pool_builder = ScrollPoolBuilder::<ScrollPooledTransaction>::default();
        let ctx = BuilderContext::<
            FullNodeTypesAdapter<
                ScrollNode,
                DatabaseMock,
                NoopProvider<ScrollChainSpec, ScrollPrimitives>,
            >,
        >::new(
            Head::default(),
            NoopProvider::new(SCROLL_MAINNET.clone()),
            manager.executor(),
            config,
        );
        (pool_builder.build_pool(&ctx).await.unwrap(), manager)
    }

    #[tokio::test]
    async fn test_validate_one_oversized_transaction() {
        // create the pool.
        let (pool, manager) = pool().await;
        let tx = ScrollTxEnvelope::Legacy(Signed::new_unchecked(
            TxLegacy { gas_limit: 21_000, ..Default::default() },
            Signature::new(U256::ZERO, U256::ZERO, false),
            Default::default(),
        ));

        // Create a pool transaction with an encoded length of 123,904 bytes.
        let pool_tx = ScrollPooledTransaction::new(
            Recovered::new_unchecked(tx, Default::default()),
            121 * 1024,
        );

        // add the transaction to the pool and expect an `OversizedData` error.
        let err = pool.add_transaction(TransactionOrigin::Local, pool_tx).await.unwrap_err();
        assert!(matches!(
            err.kind,
            PoolErrorKind::InvalidTransaction(
                InvalidPoolTransactionError::OversizedData(x, y,)
            ) if x == 121*1024 && y == 120*1024,
        ));

        // explicitly drop the manager here otherwise the `TransactionValidationTaskExecutor` will
        // drop all validation tasks.
        drop(manager);
    }

    #[tokio::test]
    async fn test_validate_one_rollup_fee_exceeds_limit() {
        // create the client.
        let handle = tokio::runtime::Handle::current();
        let manager = TaskManager::new(handle);
        let blob_store = NoopBlobStore::default();
        let signer = Default::default();
        let client =
            MockEthProvider::<ScrollPrimitives, _>::new().with_chain_spec(SCROLL_DEV.clone());
        let hash = B256::random();

        // load a header, block, signer and the L1_GAS_PRICE_ORACLE_ADDRESS storage.
        client.add_header(hash, Header::default());
        client.add_block(hash, ScrollBlock::default());
        client.add_account(signer, ExtendedAccount::new(0, U256::from(400_000)));
        client.add_account(
            L1_GAS_PRICE_ORACLE_ADDRESS,
            ExtendedAccount::new(0, U256::from(400_000)).extend_storage(
                (0u8..8).map(|k| (B256::from(U256::from(k)), U256::from(u64::MAX))),
            ),
        );

        // create the validation task.
        let validator = TransactionValidationTaskExecutor::eth_builder(client)
            .no_eip4844()
            .build_with_tasks(manager.executor(), blob_store)
            .map(|validator| {
                ScrollTransactionValidator::new(validator).require_l1_data_gas_fee(true)
            });

        // create the pool.
        let pool = ScrollTransactionPool::new(
            validator,
            CoinbaseTipOrdering::<ScrollPooledTransaction>::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        );

        // prepare a transaction with random input.
        let tx = ScrollTxEnvelope::Legacy(Signed::new_unchecked(
            TxLegacy {
                gas_limit: 55_000,
                gas_price: 7,
                input: Bytes::from(random_iter::<u8>().take(100).collect::<Vec<_>>()),
                ..Default::default()
            },
            Signature::new(U256::ZERO, U256::ZERO, false),
            Default::default(),
        ));
        let pool_tx =
            ScrollPooledTransaction::new(Recovered::new_unchecked(tx, signer), 120 * 1024);

        // add the transaction in the pool and expect to hit `InsufficientFunds` error.
        let err = pool.add_transaction(TransactionOrigin::Local, pool_tx).await.unwrap_err();
        assert!(matches!(
            err.kind,
            PoolErrorKind::InvalidTransaction(InvalidPoolTransactionError::Consensus(
                InvalidTransactionError::GasUintOverflow
            ))
        ));

        // explicitly drop the manager here otherwise the `TransactionValidationTaskExecutor` will
        // drop all validation tasks.
        drop(manager);
    }

    #[tokio::test]
    async fn test_validate_one_rollup_fee_exceeds_balance() {
        // create the client.
        let handle = tokio::runtime::Handle::current();
        let manager = TaskManager::new(handle);
        let blob_store = NoopBlobStore::default();
        let signer = Default::default();
        let client =
            MockEthProvider::<ScrollPrimitives, _>::new().with_chain_spec(SCROLL_DEV.clone());
        let hash = B256::random();

        // load a header, block, signer and the L1_GAS_PRICE_ORACLE_ADDRESS storage.
        client.add_header(hash, Header::default());
        client.add_block(hash, ScrollBlock::default());
        client.add_account(signer, ExtendedAccount::new(0, U256::from(400_000)));
        client.add_account(
            L1_GAS_PRICE_ORACLE_ADDRESS,
            ExtendedAccount::new(0, U256::from(400_000)).extend_storage(
                (0u8..8).map(|k| (B256::from(U256::from(k)), U256::from(u32::MAX))),
            ),
        );

        // create the validation task.
        let validator = TransactionValidationTaskExecutor::eth_builder(client)
            .no_eip4844()
            .build_with_tasks(manager.executor(), blob_store)
            .map(|validator| {
                ScrollTransactionValidator::new(validator).require_l1_data_gas_fee(true)
            });

        // create the pool.
        let pool = ScrollTransactionPool::new(
            validator,
            CoinbaseTipOrdering::<ScrollPooledTransaction>::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        );

        // prepare a transaction with random input.
        let tx = ScrollTxEnvelope::Legacy(Signed::new_unchecked(
            TxLegacy {
                gas_limit: 55_000,
                gas_price: 7,
                input: Bytes::from(random_iter::<u8>().take(100).collect::<Vec<_>>()),
                ..Default::default()
            },
            Signature::new(U256::ZERO, U256::ZERO, false),
            Default::default(),
        ));
        let pool_tx =
            ScrollPooledTransaction::new(Recovered::new_unchecked(tx, signer), 120 * 1024);

        // add the transaction in the pool and expect to hit `InsufficientFunds` error.
        let err = pool.add_transaction(TransactionOrigin::Local, pool_tx).await.unwrap_err();
        assert!(matches!(
            err.kind,
            PoolErrorKind::InvalidTransaction(
                InvalidPoolTransactionError::Consensus(InvalidTransactionError::InsufficientFunds(GotExpectedBoxed(expected)))
            ) if *expected == GotExpected{ got: U256::from(400000), expected: U256::from(483673629772436u64) }
        ));

        // explicitly drop the manager here otherwise the `TransactionValidationTaskExecutor` will
        // drop all validation tasks.
        drop(manager);
    }

    #[tokio::test]
    async fn test_validate_one_disallow_l1_messages() {
        // create the pool.
        let (pool, manager) = pool().await;
        let tx = ScrollTxEnvelope::L1Message(Sealed::new_unchecked(
            TxL1Message::default(),
            B256::default(),
        ));

        // Create a pool transaction with the L1 message.
        let pool_tx =
            ScrollPooledTransaction::new(Recovered::new_unchecked(tx, Default::default()), 0);

        // add the transaction to the pool and expect an `OversizedData` error.
        let err = pool.add_transaction(TransactionOrigin::Local, pool_tx).await.unwrap_err();
        assert!(matches!(
            err.kind,
            PoolErrorKind::InvalidTransaction(InvalidPoolTransactionError::Consensus(
                InvalidTransactionError::TxTypeNotSupported
            ))
        ));

        // explicitly drop the manager here otherwise the `TransactionValidationTaskExecutor` will
        // drop all validation tasks.
        drop(manager);
    }

    #[test]
    fn test_pool_builder_with_require_l1_data_fee_buffer() {
        // Test that the builder method correctly sets the flag
        let pool_builder = ScrollPoolBuilder::<ScrollPooledTransaction>::default();
        assert!(!pool_builder.require_l1_data_fee_buffer);

        let pool_builder = pool_builder.with_require_l1_data_fee_buffer(true);
        assert!(pool_builder.require_l1_data_fee_buffer);

        let pool_builder = pool_builder.with_require_l1_data_fee_buffer(false);
        assert!(!pool_builder.require_l1_data_fee_buffer);
    }

    #[tokio::test]
    async fn test_l1_data_fee_buffer_validation() {
        // Test that the L1 data fee buffer feature correctly validates transactions:
        // - With buffer enabled: rejects when balance < L2_cost + 2*L1_cost
        // - With buffer disabled: accepts when balance >= L2_cost + 1*L1_cost
        //
        // Both scenarios use identical setup to prove only the buffer flag differs.

        // Shared test constants
        let signer: alloy_primitives::Address = Default::default();
        let balance = U256::from(500_000_000_000_000u64); // 500 Twei
        let gas_limit = 55_000u64;
        let gas_price = 7u128;
        let tx_input = Bytes::from(random_iter::<u8>().take(100).collect::<Vec<_>>());

        // Helper to create a client with identical state
        let client =
            MockEthProvider::<ScrollPrimitives, _>::new().with_chain_spec(SCROLL_DEV.clone());
        let hash = B256::random();
        client.add_header(hash, Header::default());
        client.add_block(hash, ScrollBlock::default());
        // Balance covers L2_cost + 1*L1_cost but NOT L2_cost + 2*L1_cost
        // With u32::MAX storage values, L1 cost is ~483 Twei.
        // max L2_cost = 55,000 * 7 = 385,000 Wei.
        client.add_account(signer, ExtendedAccount::new(0, balance));
        client.add_account(
            L1_GAS_PRICE_ORACLE_ADDRESS,
            ExtendedAccount::new(0, U256::ZERO).extend_storage(
                (0u8..8).map(|k| (B256::from(U256::from(k)), U256::from(u32::MAX))),
            ),
        );

        // Helper to create a transaction with identical parameters
        let tx = ScrollTxEnvelope::Legacy(Signed::new_unchecked(
            TxLegacy { gas_limit, gas_price, input: tx_input.clone(), ..Default::default() },
            Signature::new(U256::ZERO, U256::ZERO, false),
            Default::default(),
        ));
        let tx = ScrollPooledTransaction::new(Recovered::new_unchecked(tx, signer), 200);

        let handle = tokio::runtime::Handle::current();
        let manager = TaskManager::new(handle);

        // Test 1: With L1 data fee buffer ENABLED - should reject (requires 2x L1 cost)
        let validator = TransactionValidationTaskExecutor::eth_builder(client.clone())
            .no_eip4844()
            .build_with_tasks(manager.executor(), NoopBlobStore::default())
            .map(|validator| {
                ScrollTransactionValidator::new(validator)
                    .require_l1_data_gas_fee(true)
                    .require_l1_data_fee_buffer(true)
            });

        let pool = ScrollTransactionPool::new(
            validator,
            CoinbaseTipOrdering::<ScrollPooledTransaction>::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        );

        let err = pool.add_transaction(TransactionOrigin::Local, tx.clone()).await.unwrap_err();
        assert!(matches!(
            err.kind,
            PoolErrorKind::InvalidTransaction(
                InvalidPoolTransactionError::Consensus(InvalidTransactionError::InsufficientFunds(GotExpectedBoxed(expected)))
            ) if *expected == GotExpected{ got: balance, expected: U256::from(967347259159872u64) }
        ));

        // Test 2: With L1 data fee buffer DISABLED - should accept (only requires 1x L1 cost)
        let validator = TransactionValidationTaskExecutor::eth_builder(client)
            .no_eip4844()
            .build_with_tasks(manager.executor(), NoopBlobStore::default())
            .map(|validator| {
                ScrollTransactionValidator::new(validator)
                    .require_l1_data_gas_fee(true)
                    .require_l1_data_fee_buffer(false)
            });

        let pool = ScrollTransactionPool::new(
            validator,
            CoinbaseTipOrdering::<ScrollPooledTransaction>::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        );

        let result = pool.add_transaction(TransactionOrigin::Local, tx).await;
        assert!(
            result.is_ok(),
            "Expected transaction to be accepted without buffer, got: {result:?}"
        );

        drop(manager);
    }
}
