use reth_chainspec::{ChainSpec, Head};
use reth_evm::{ConfigureEvm, ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_primitives::{transaction::FillTxEnv, TransactionSigned};
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
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::Header;
    use reth_chainspec::NamedChain::Scroll;
    use revm::primitives::{address, SpecId, B256};

    #[test]
    fn test_spec_at_head() {
        // TODO (scroll): change this to `ScrollChainSpecBuilder::mainnet()`.
        let config = ScrollEvmConfig::new(ChainSpec::default().into());

        // prepare all fork heads
        let euclid_head = &Head { number: u64::MAX, ..Default::default() };
        let curie_head = &Head { number: 7096836, ..Default::default() };
        let bernouilli_head = &Head { number: 5220340, ..Default::default() };
        let pre_bernouilli_head = &Head { number: 0, ..Default::default() };

        // check correct spec id
        assert_eq!(config.spec_id_at_head(euclid_head), SpecId::EUCLID);
        assert_eq!(config.spec_id_at_head(curie_head), SpecId::CURIE);
        assert_eq!(config.spec_id_at_head(bernouilli_head), SpecId::BERNOULLI);
        assert_eq!(config.spec_id_at_head(pre_bernouilli_head), SpecId::PRE_BERNOULLI);
    }

    #[test]
    fn test_fill_cfg_env() {
        // TODO (scroll): change this to `ScrollChainSpecBuilder::mainnet()`.
        let config = ScrollEvmConfig::new(ChainSpec::default().into());

        // euclid header
        let mut cfg_env = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let euclid_header = Header { number: u64::MAX, ..Default::default() };

        // fill cfg env
        config.fill_cfg_env(&mut cfg_env, &euclid_header, U256::ZERO);

        // check correct cfg env
        assert_eq!(cfg_env.chain_id, Scroll as u64);
        assert_eq!(cfg_env.perf_analyse_created_bytecodes, AnalysisKind::Analyse);
        assert_eq!(cfg_env.handler_cfg.spec_id, SpecId::EUCLID);
        assert!(cfg_env.handler_cfg.is_scroll);

        // curie
        let mut cfg_env = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let curie_header = Header { number: 7096836, ..Default::default() };

        // fill cfg env
        config.fill_cfg_env(&mut cfg_env, &curie_header, U256::ZERO);

        // check correct cfg env
        assert_eq!(cfg_env.chain_id, Scroll as u64);
        assert_eq!(cfg_env.perf_analyse_created_bytecodes, AnalysisKind::Analyse);
        assert_eq!(cfg_env.handler_cfg.spec_id, SpecId::CURIE);
        assert!(cfg_env.handler_cfg.is_scroll);

        // bernouilli
        let mut cfg_env = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let bernouilli_header = Header { number: 5220340, ..Default::default() };

        // fill cfg env
        config.fill_cfg_env(&mut cfg_env, &bernouilli_header, U256::ZERO);

        // check correct cfg env
        assert_eq!(cfg_env.chain_id, Scroll as u64);
        assert_eq!(cfg_env.perf_analyse_created_bytecodes, AnalysisKind::Analyse);
        assert_eq!(cfg_env.handler_cfg.spec_id, SpecId::BERNOULLI);
        assert!(cfg_env.handler_cfg.is_scroll);

        // pre-bernouilli
        let mut cfg_env = CfgEnvWithHandlerCfg::new(Default::default(), Default::default());
        let pre_bernouilli_header = Header { number: 0, ..Default::default() };

        // fill cfg env
        config.fill_cfg_env(&mut cfg_env, &pre_bernouilli_header, U256::ZERO);

        // check correct cfg env
        assert_eq!(cfg_env.chain_id, Scroll as u64);
        assert_eq!(cfg_env.perf_analyse_created_bytecodes, AnalysisKind::Analyse);
        assert_eq!(cfg_env.handler_cfg.spec_id, SpecId::PRE_BERNOULLI);
        assert!(cfg_env.handler_cfg.is_scroll);
    }

    #[test]
    fn test_fill_block_env() {
        // TODO (scroll): change this to `ScrollChainSpecBuilder::mainnet()`.
        let config = ScrollEvmConfig::new(ChainSpec::default().into());
        let mut block_env = BlockEnv::default();

        // curie header
        let header = Header {
            number: 7096836,
            beneficiary: Address::random(),
            timestamp: 1719994277,
            mix_hash: B256::random(),
            base_fee_per_gas: Some(155157341),
            gas_limit: 10000000,
            ..Default::default()
        };

        // fill block env
        config.fill_block_env(&mut block_env, &header, true);

        // verify block env correctly updated
        // TODO (scroll): replace with `config.chain_spec.fee_vault_address.unwrap()`
        const FEE_VAULT_ADDRESS: Address = address!("5300000000000000000000000000000000000005");
        let expected = BlockEnv {
            number: U256::from(header.number),
            coinbase: FEE_VAULT_ADDRESS,
            timestamp: U256::from(header.timestamp),
            prevrandao: Some(header.mix_hash),
            difficulty: U256::ZERO,
            basefee: U256::from(header.base_fee_per_gas.unwrap_or_default()),
            gas_limit: U256::from(header.gas_limit),
            ..Default::default()
        };
        assert_eq!(block_env, expected)
    }

    #[test]
    fn test_next_cfg_and_block_env() -> eyre::Result<()> {
        // TODO (scroll): change this to `ScrollChainSpecBuilder::mainnet()`.
        let config = ScrollEvmConfig::new(ChainSpec::default().into());

        // pre curie header
        let header = Header {
            number: 7096835,
            beneficiary: Address::random(),
            timestamp: 1719994274,
            mix_hash: B256::random(),
            base_fee_per_gas: None,
            gas_limit: 10000000,
            ..Default::default()
        };

        // curie block attributes
        let attributes = NextBlockEnvAttributes {
            timestamp: 1719994277,
            suggested_fee_recipient: Address::random(),
            prev_randao: B256::random(),
        };

        // get next cfg env and block env
        let (cfg_env, block_env) = config.next_cfg_and_block_env(&header, attributes)?;

        // verify cfg env
        assert_eq!(cfg_env.chain_id, Scroll as u64);
        assert_eq!(cfg_env.handler_cfg.spec_id, SpecId::CURIE);
        assert!(cfg_env.handler_cfg.is_scroll);

        // verify block env
        // TODO (scroll): replace with `config.chain_spec.fee_vault_address.unwrap()`
        const FEE_VAULT_ADDRESS: Address = address!("5300000000000000000000000000000000000005");
        let expected = BlockEnv {
            number: U256::from(header.number + 1),
            coinbase: FEE_VAULT_ADDRESS,
            timestamp: U256::from(attributes.timestamp),
            prevrandao: Some(attributes.prev_randao),
            difficulty: U256::ZERO,
            // TODO (scroll): update once the Scroll chain spec is available
            // TODO (scroll): this shouldn't be 0 at curie fork
            basefee: U256::ZERO,
            gas_limit: U256::from(header.gas_limit),
            ..Default::default()
        };
        assert_eq!(block_env, expected);

        Ok(())
    }
}
