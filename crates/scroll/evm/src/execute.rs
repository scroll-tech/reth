//! Implementation of the [`BlockExecutionStrategy`] for Scroll.

use crate::{HardForkError, ScrollBlockExecutionError};
use alloy_consensus::{Header, Transaction};
use alloy_eips::{eip2718::Encodable2718, eip7685::Requests};
use reth_chainspec::{ChainSpec, EthereumHardfork, EthereumHardforks};
use reth_consensus::ConsensusError;
use reth_evm::{
    execute::{BlockExecutionStrategy, BlockValidationError, ExecuteOutput, ProviderError},
    ConfigureEvm, ConfigureEvmEnv,
};
use reth_primitives::{
    gas_spent_by_transactions, BlockWithSenders, GotExpected, InvalidTransactionError, Receipt,
};
use reth_revm::primitives::{CfgEnvWithHandlerCfg, U256};
use reth_scroll_consensus::apply_curie_hard_fork;
use reth_scroll_execution::FinalizeExecution;
use revm::{
    db::BundleState,
    primitives::{bytes::BytesMut, BlockEnv, EnvWithHandlerCfg, ResultAndState},
    Database, DatabaseCommit, State,
};
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

/// The Scroll block execution strategy.
#[derive(Debug)]
pub struct ScrollExecutionStrategy<DB, EvmConfig> {
    /// Chain specification.
    chain_spec: Arc<ChainSpec>,
    /// Evm configuration.
    evm_config: EvmConfig,
    /// Current state for the execution.
    state: State<DB>,
}

impl<DB, EvmConfig> ScrollExecutionStrategy<DB, EvmConfig> {
    /// Returns an instance of [`ScrollExecutionStrategy`].
    pub const fn new(chain_spec: Arc<ChainSpec>, evm_config: EvmConfig, state: State<DB>) -> Self {
        Self { chain_spec, evm_config, state }
    }
}

impl<DB, EvmConfig> ScrollExecutionStrategy<DB, EvmConfig>
where
    EvmConfig: ConfigureEvmEnv<Header = Header>,
{
    /// Configures a new evm configuration and block environment for the given block.
    ///
    /// # Caution
    ///
    /// This does not initialize the tx environment.
    fn evm_env_for_block(&self, header: &Header, total_difficulty: U256) -> EnvWithHandlerCfg {
        let mut cfg = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let mut block_env = BlockEnv::default();
        self.evm_config.fill_cfg_and_block_env(&mut cfg, &mut block_env, header, total_difficulty);

        EnvWithHandlerCfg::new_with_cfg_env(cfg, block_env, Default::default())
    }
}

impl<DB, EvmConfig> BlockExecutionStrategy<DB> for ScrollExecutionStrategy<DB, EvmConfig>
where
    DB: Database<Error: Into<ProviderError> + Display>,
    State<DB>: FinalizeExecution<Output = BundleState>,
    EvmConfig: ConfigureEvm<Header = Header>,
{
    type Error = ScrollBlockExecutionError;

    fn apply_pre_execution_changes(
        &mut self,
        block: &BlockWithSenders,
        _total_difficulty: U256,
    ) -> Result<(), Self::Error> {
        // TODO (scroll): update to the Scroll chain spec
        // TODO (scroll): update to the Curie hardfork
        if self.chain_spec.fork(EthereumHardfork::Dao).transitions_at_block(block.number) {
            if let Err(err) = apply_curie_hard_fork(&mut self.state) {
                tracing::debug!(%err, "failed to apply curie hardfork");
                return Err(HardForkError::Curie.into());
            };
        }

        Ok(())
    }

    fn execute_transactions(
        &mut self,
        block: &BlockWithSenders,
        total_difficulty: U256,
    ) -> Result<ExecuteOutput, Self::Error> {
        let env = self.evm_env_for_block(&block.header, total_difficulty);
        let mut evm = self.evm_config.evm_with_env(&mut self.state, env);

        let mut cumulative_gas_used = 0;
        let mut receipts = Vec::with_capacity(block.body.transactions.len());

        for (sender, transaction) in block.transactions_with_sender() {
            // The sum of the transaction’s gas limit and the gas utilized in this block prior,
            // must be no greater than the block’s gasLimit.
            let block_available_gas = block.header.gas_limit - cumulative_gas_used;
            if transaction.gas_limit() > block_available_gas {
                return Err(BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                    transaction_gas_limit: transaction.gas_limit(),
                    block_available_gas,
                }
                .into())
            }

            if transaction.is_eip4844() {
                return Err(ConsensusError::InvalidTransaction(
                    InvalidTransactionError::Eip4844Disabled,
                )
                .into())
            }

            self.evm_config.fill_tx_env(evm.tx_mut(), transaction, *sender);
            if transaction.is_l1_message() {
                evm.context.evm.env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }

            // RLP encode the transaction following eip 2718
            let mut buf = BytesMut::with_capacity(transaction.encode_2718_len());
            transaction.encode_2718(&mut buf);
            let transaction_rlp_bytes = buf.freeze();
            evm.context.evm.env.tx.scroll.rlp_bytes = Some(transaction_rlp_bytes.into());
            evm.context.evm.env.tx.scroll.is_l1_msg = transaction.is_l1_message();

            // execute the transaction and commit the result to the database
            let ResultAndState { result, state } =
                evm.transact().map_err(|err| BlockValidationError::EVM {
                    hash: transaction.recalculate_hash(),
                    error: Box::new(err.map_db_err(|e| e.into())),
                })?;

            evm.db_mut().commit(state);

            let l1_fee = if transaction.is_l1_message() {
                U256::ZERO
            } else {
                // compute l1 fee for all non-l1 transaction
                let l1_block_info =
                    evm.context.evm.inner.l1_block_info.as_ref().ok_or_else(|| {
                        ScrollBlockExecutionError::l1_fee("missing l1 block info")
                    })?;
                let transaction_rlp_bytes =
                    evm.context.evm.env.tx.scroll.rlp_bytes.as_ref().ok_or_else(|| {
                        ScrollBlockExecutionError::l1_fee("missing transaction rlp bytes")
                    })?;
                l1_block_info.calculate_tx_l1_cost(transaction_rlp_bytes, evm.handler.cfg.spec_id)
            };

            cumulative_gas_used += result.gas_used();

            receipts.push(Receipt {
                tx_type: transaction.tx_type(),
                success: result.is_success(),
                cumulative_gas_used,
                logs: result.into_logs(),
                l1_fee,
            })
        }

        Ok(ExecuteOutput { receipts, gas_used: cumulative_gas_used })
    }

    fn apply_post_execution_changes(
        &mut self,
        _block: &BlockWithSenders,
        _total_difficulty: U256,
        _receipts: &[Receipt],
    ) -> Result<Requests, Self::Error> {
        Ok(Default::default())
    }

    fn state_ref(&self) -> &State<DB> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<DB> {
        &mut self.state
    }

    fn validate_block_post_execution(
        &self,
        block: &BlockWithSenders,
        receipts: &[Receipt],
        _requests: &Requests,
    ) -> Result<(), ConsensusError> {
        // verify the block gas used
        let cumulative_gas_used = receipts.last().map(|r| r.cumulative_gas_used).unwrap_or(0);
        if block.gas_used != cumulative_gas_used {
            return Err(ConsensusError::BlockGasUsed {
                gas: GotExpected { got: cumulative_gas_used, expected: block.gas_used },
                gas_spent_by_tx: gas_spent_by_transactions(receipts),
            });
        }

        // verify the receipts logs bloom and root
        if self.chain_spec.is_byzantium_active_at_block(block.header.number) {
            if let Err(error) = reth_ethereum_consensus::verify_receipts(
                block.header.receipts_root,
                block.header.logs_bloom,
                receipts,
            ) {
                tracing::debug!(
                    %error,
                    ?receipts,
                    header_receipt_root = ?block.header.receipts_root,
                    header_bloom = ?block.header.logs_bloom,
                    "failed to verify receipts"
                );
                return Err(error);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrollEvmConfig;
    use reth_chainspec::ChainSpecBuilder;
    use reth_primitives::{Block, BlockBody, TransactionSigned};
    use reth_scroll_consensus::{
        CURIE_L1_GAS_PRICE_ORACLE_BYTECODE, CURIE_L1_GAS_PRICE_ORACLE_STORAGE,
        L1_GAS_PRICE_ORACLE_ADDRESS,
    };
    use revm::{
        db::states::{bundle_state::BundleRetention, StorageSlot},
        primitives::{Address, TxType, B256},
        Bytecode, EmptyDBTyped, TxKind,
    };

    fn strategy() -> ScrollExecutionStrategy<EmptyDBTyped<ProviderError>, ScrollEvmConfig> {
        // TODO (scroll): change this to `ScrollChainSpecBuilder::mainnet()`.
        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());
        let config = ScrollEvmConfig::new(chain_spec.clone());
        let db = EmptyDBTyped::<ProviderError>::new();
        let state =
            State::builder().with_database(db).with_bundle_update().without_state_clear().build();

        ScrollExecutionStrategy::new(chain_spec, config, state)
    }

    fn transaction(typ: TxType, gas_limit: u64) -> TransactionSigned {
        let pk = B256::random();
        let transaction = match typ {
            TxType::BlobTx => reth_primitives::Transaction::Eip4844(alloy_consensus::TxEip4844 {
                gas_limit,
                to: Address::ZERO,
                ..Default::default()
            }),
            _ => reth_primitives::Transaction::Legacy(alloy_consensus::TxLegacy {
                gas_limit,
                to: TxKind::Call(Address::ZERO),
                ..Default::default()
            }),
        };
        let signature = reth_primitives::sign_message(pk, transaction.signature_hash()).unwrap();
        reth_primitives::TransactionSigned::new_unhashed(transaction, signature)
    }

    #[test]
    fn test_apply_pre_execution_changes_at_curie_block() -> eyre::Result<()> {
        // init strategy
        let mut strategy = strategy();

        // init curie transition block
        let curie_block = BlockWithSenders {
            block: Block {
                header: Header { number: 7096836, ..Default::default() },
                ..Default::default()
            },
            senders: vec![],
        };

        // apply pre execution change
        strategy.apply_pre_execution_changes(&curie_block, U256::ZERO)?;

        // take bundle
        let mut state = strategy.state;
        state.merge_transitions(BundleRetention::Reverts);
        let bundle = state.take_bundle();

        // assert oracle contract contains updated bytecode and storage
        let oracle = bundle.state.get(&L1_GAS_PRICE_ORACLE_ADDRESS).unwrap().clone();
        let bytecode = Bytecode::new_raw(CURIE_L1_GAS_PRICE_ORACLE_BYTECODE);
        assert_eq!(oracle.info.unwrap().code.unwrap(), bytecode);

        // check oracle storage changeset
        let mut storage = oracle.storage.into_iter().collect::<Vec<(U256, StorageSlot)>>();
        storage.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (got, expected) in storage.into_iter().zip(CURIE_L1_GAS_PRICE_ORACLE_STORAGE) {
            assert_eq!(got.0, expected.0);
            assert_eq!(got.1, StorageSlot { present_value: expected.1, ..Default::default() });
        }

        Ok(())
    }

    #[test]
    fn test_apply_pre_execution_changes_not_at_curie_block() -> eyre::Result<()> {
        // init strategy
        let mut strategy = strategy();

        // init curie transition block
        let curie_block = BlockWithSenders {
            block: Block {
                header: Header { number: 7096837, ..Default::default() },
                ..Default::default()
            },
            senders: vec![],
        };

        // apply pre execution change
        strategy.apply_pre_execution_changes(&curie_block, U256::ZERO)?;

        // take bundle
        let mut state = strategy.state;
        state.merge_transitions(BundleRetention::Reverts);
        let bundle = state.take_bundle();

        // assert oracle contract contains updated bytecode and storage
        let oracle = bundle.state.get(&L1_GAS_PRICE_ORACLE_ADDRESS);
        assert!(oracle.is_none());

        Ok(())
    }

    #[test]
    fn test_execute_transaction_exceeds_block_gas_limit() -> eyre::Result<()> {
        // init strategy
        let mut strategy = strategy();

        // prepare transactions exceeding block gas limit
        let gas_limit = 10_000_000;
        let transaction = transaction(TxType::Legacy, gas_limit + 1);
        let senders = vec![transaction.recover_signer().unwrap()];
        let block = BlockWithSenders {
            block: Block {
                header: Header { number: 7096837, gas_limit, ..Default::default() },
                body: BlockBody { transactions: vec![transaction], ..Default::default() },
            },
            senders: senders.clone(),
        };

        // load accounts in state
        strategy.state.insert_account(Address::ZERO, Default::default());
        strategy.state.insert_account(L1_GAS_PRICE_ORACLE_ADDRESS, Default::default());
        for add in senders {
            strategy.state.insert_account(add, Default::default());
        }

        let res = strategy.execute_transactions(&block, U256::ZERO);
        assert_eq!(
            res.unwrap_err().to_string(),
            "transaction gas limit 10000001 is more than blocks available gas 10000000"
        );

        Ok(())
    }

    #[test]
    fn test_execute_transaction_eip4844() -> eyre::Result<()> {
        // init strategy
        let mut strategy = strategy();

        // prepare transactions exceeding block gas limit
        let transaction = transaction(TxType::BlobTx, 1_000);
        let senders = vec![transaction.recover_signer().unwrap()];
        let block = BlockWithSenders {
            block: Block {
                header: Header { number: 7096837, gas_limit: 10_000_000, ..Default::default() },
                body: BlockBody { transactions: vec![transaction], ..Default::default() },
            },
            senders: senders.clone(),
        };

        // load accounts in state
        strategy.state.insert_account(Address::ZERO, Default::default());
        strategy.state.insert_account(L1_GAS_PRICE_ORACLE_ADDRESS, Default::default());
        for add in senders {
            strategy.state.insert_account(add, Default::default());
        }

        let res = strategy.execute_transactions(&block, U256::ZERO);
        assert_eq!(res.unwrap_err().to_string(), "EIP-4844 transactions are disabled");

        Ok(())
    }
}
