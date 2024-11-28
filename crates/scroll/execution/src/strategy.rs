//! Implementation of the [`BlockExecutionStrategy`] for Scroll.

use crate::{HardForkError, ScrollBlockExecutionError};
use alloy_consensus::{Header, Transaction};
use alloy_eips::eip7685::Requests;
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
use revm::{
    primitives::{BlockEnv, EnvWithHandlerCfg, ResultAndState},
    Database, DatabaseCommit, State,
};
use std::fmt::{Debug, Display};

/// The Scroll block execution strategy.
#[derive(Debug)]
pub struct ScrollExecutionStrategy<DB, EvmConfig> {
    /// Chain specification.
    chain_spec: ChainSpec,
    /// Evm configuration.
    // TODO (scroll): EvmConfig should set the correct coinbase in `fill_block_env`.
    // TODO (scroll): Should we have a ScrollConfig?
    evm_config: EvmConfig,
    /// Current state for the execution.
    state: State<DB>,
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

            // TODO (scroll): verify the logic from the stateless block verifier
            // https://github.com/scroll-tech/stateless-block-verifier/blob/master/crates/core/src/executor/mod.rs#L71
            self.evm_config.fill_tx_env(evm.tx_mut(), transaction, *sender);
            if transaction.is_l1_messaging() {
                evm.context.evm.env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }

            let ResultAndState { result, state } =
                evm.transact().map_err(|err| BlockValidationError::EVM {
                    hash: transaction.recalculate_hash(),
                    error: Box::new(err.map_db_err(|e| e.into())),
                })?;

            evm.db_mut().commit(state);

            // TODO (scroll): uncomment once we have the scroll/revm fork integrated
            let l1_fee = U256::ZERO;
            // if !transaction.is_l1_msg() {
            //     let mut rlp_bytes = BytesMut::new();
            //     transaction.eip2718_encode(&mut rlp_bytes, transaction.signature());
            //     let l1_block_info =
            //         evm.context.evm.inner.l1_block_info.as_ref().expect("l1 block info exists");
            //     l1_fee = l1_block_info.calculate_tx_l1_cost(rlp_bytes, evm.handler.cfg.spec_id);
            // }

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
