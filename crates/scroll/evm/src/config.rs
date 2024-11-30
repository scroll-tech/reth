use reth_chainspec::{ChainSpec, Head};
use reth_evm::{ConfigureEvm, ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_primitives::TransactionSigned;
use reth_primitives_traits::FillTxEnv;
use reth_revm::{inspector_handle_register, Database, Evm, GetInspector, TxEnv};
use revm::{
    precompile::{Address, Bytes},
    primitives::{
        AnalysisKind, BlockEnv, CfgEnv, CfgEnvWithHandlerCfg, Env, HandlerCfg, SpecId, U256,
    },
    EvmBuilder,
};
use std::{convert::Infallible, sync::Arc};

/// Scroll EVM configuration.
#[derive(Clone, Debug)]
pub struct ScrollEvmConfig {
    /// The chain spec for Scroll.
    // TODO (scroll): update to ScrollChainSpec.
    chain_spec: Arc<ChainSpec>,
}

impl ScrollEvmConfig {
    /// Returns a new instance of [`ScrollEvmConfig`].
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }

    /// Returns the spec id at the given head.
    pub fn spec_id_at_head(&self, _head: &Head) -> SpecId {
        // TODO (scroll): uncomment once the Scroll chain spec is available
        // let chain_spec = &self.chain_spec;
        // if chain_spec.fork(ScrollHardfork::Euclid).active_at_head(head) {
        //     SpecId::EUCLID
        // } else if chain_spec.fork(ScrollHardfork::Curie).active_at_head(head) {
        //     SpecId::CURIE
        // } else if chain_spec.fork(ScrollHardfork::Bernouilli).active_at_head(head) {
        //     SpecId::BERNOULLI
        // } else {
        //     SpecId::PRE_BERNOULLI
        // }
        SpecId::PRE_BERNOULLI
    }
}

impl ConfigureEvm for ScrollEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        EvmBuilder::default().with_db(db).scroll().build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .scroll()
            .append_handler_register(inspector_handle_register)
            .build()
    }

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
}

impl ConfigureEvmEnv for ScrollEvmConfig {
    type Header = alloy_consensus::Header;
    type Error = Infallible;

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        transaction.fill_tx_env(tx_env, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        _env: &mut Env,
        _caller: Address,
        _contract: Address,
        _data: Bytes,
    ) {
        /* noop */
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        total_difficulty: U256,
    ) {
        let spec_id = self.spec_id_at_head(&Head {
            number: header.number,
            timestamp: header.timestamp,
            difficulty: header.difficulty,
            total_difficulty,
            ..Default::default()
        });

        cfg_env.handler_cfg.spec_id = spec_id;
        cfg_env.handler_cfg.is_scroll = true;

        cfg_env.chain_id = self.chain_spec.chain().id();
        cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Analyse;
    }

    fn fill_block_env(&self, block_env: &mut BlockEnv, header: &Self::Header, after_merge: bool) {
        block_env.number = U256::from(header.number);
        block_env.coinbase = header.beneficiary;
        // TODO (scroll): uncomment once the Scroll chain spec is available
        // if let Some(vault_address) = self.chain_spec.fee_vault_address {
        //    block_env.coinbase = vault_address;
        // }
        block_env.timestamp = U256::from(header.timestamp);
        if after_merge {
            block_env.prevrandao = Some(header.mix_hash);
            block_env.difficulty = U256::ZERO;
        } else {
            block_env.difficulty = header.difficulty;
            block_env.prevrandao = None;
        }
        block_env.basefee = U256::from(header.base_fee_per_gas.unwrap_or_default());
        block_env.gas_limit = U256::from(header.gas_limit);
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Self::Error> {
        // configure evm env based on parent block
        let cfg = CfgEnv::default().with_chain_id(self.chain_spec.chain().id());

        // fetch spec id from next head number and timestamp
        let spec_id = self.spec_id_at_head(&Head {
            number: parent.number + 1,
            timestamp: attributes.timestamp,
            ..Default::default()
        });

        let coinbase = attributes.suggested_fee_recipient;
        // TODO (scroll): uncomment once the Scroll chain spec is available
        // if let Some(vault_address) = self.chain_spec.fee_vault_address {
        //    block_env.coinbase = vault_address;
        // }

        let block_env = BlockEnv {
            number: U256::from(parent.number + 1),
            coinbase,
            timestamp: U256::from(attributes.timestamp),
            difficulty: U256::ZERO,
            prevrandao: Some(attributes.prev_randao),
            gas_limit: U256::from(parent.gas_limit),
            // calculate basefee based on parent block's gas usage
            basefee: U256::ZERO,
            // TODO (scroll): uncomment once the Scroll chain spec is available
            // self.chain_spec.next_block_base_fee(parent, attributes.timestamp)?,
            blob_excess_gas_and_price: None,
        };

        let cfg_with_handler_cfg = CfgEnvWithHandlerCfg {
            cfg_env: cfg,
            handler_cfg: HandlerCfg { spec_id, is_scroll: true },
        };

        Ok((cfg_with_handler_cfg, block_env))
    }
}
