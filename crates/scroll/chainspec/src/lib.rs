//! OP-Reth chain specs.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod constants;
mod dev;
mod genesis;
mod scroll;
mod scroll_sepolia;

use crate::genesis::ScrollChainInfo;
use alloc::{boxed::Box, vec::Vec};
use alloy_chains::Chain;
use alloy_consensus::Header;
use alloy_genesis::Genesis;
use alloy_primitives::{B256, U256};
use derive_more::{Constructor, Deref, Display, From, Into};
pub use dev::SCROLL_DEV;
#[cfg(not(feature = "std"))]
pub(crate) use once_cell::sync::Lazy as LazyLock;
use reth_chainspec::{
    BaseFeeParams, ChainSpec, ChainSpecBuilder, DepositContract, EthChainSpec, EthereumHardforks,
    ForkFilter, ForkId, Hardforks, Head,
};
use reth_ethereum_forks::{ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};
use reth_network_peers::NodeRecord;
use reth_scroll_forks::ScrollHardforks;
pub use scroll::SCROLL_MAINNET;
pub use scroll_sepolia::SCROLL_SEPOLIA;
#[cfg(feature = "std")]
pub(crate) use std::sync::LazyLock;

/// Chain spec builder for a Scroll chain.
#[derive(Debug, Default, From)]
pub struct ScrollChainSpecBuilder {
    /// [`ChainSpecBuilder`]
    inner: ChainSpecBuilder,
}

impl ScrollChainSpecBuilder {
    /// Construct a new builder from the scroll mainnet chain spec.
    pub fn scroll_mainnet() -> Self {
        let mut inner = ChainSpecBuilder::default()
            .chain(SCROLL_MAINNET.chain)
            .genesis(SCROLL_MAINNET.genesis.clone());
        let forks = SCROLL_MAINNET.hardforks.clone();
        inner = inner.with_forks(forks);

        Self { inner }
    }
}

impl ScrollChainSpecBuilder {
    /// Set the chain ID
    pub fn chain(mut self, chain: Chain) -> Self {
        self.inner = self.inner.chain(chain);
        self
    }

    /// Set the genesis block.
    pub fn genesis(mut self, genesis: Genesis) -> Self {
        self.inner = self.inner.genesis(genesis);
        self
    }

    /// Add the given fork with the given activation condition to the spec.
    pub fn with_fork<H: Hardfork>(mut self, fork: H, condition: ForkCondition) -> Self {
        self.inner = self.inner.with_fork(fork, condition);
        self
    }

    /// Add the given forks with the given activation condition to the spec.
    pub fn with_forks(mut self, forks: ChainHardforks) -> Self {
        self.inner = self.inner.with_forks(forks);
        self
    }

    /// Remove the given fork from the spec.
    pub fn without_fork(mut self, fork: reth_scroll_forks::ScrollHardFork) -> Self {
        self.inner = self.inner.without_fork(fork);
        self
    }

    /// Enable Bernoulli at genesis
    pub fn bernoulli_activated(mut self) -> Self {
        self.inner = self.inner.cancun_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardFork::Bernoulli, ForkCondition::Block(0));
        self
    }

    /// Enable Curie at genesis
    pub fn curie_activated(mut self) -> Self {
        self = self.bernoulli_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardFork::Curie, ForkCondition::Timestamp(0));
        self
    }

    /// Enable Darwin at genesis
    pub fn darwin_activated(mut self) -> Self {
        self = self.curie_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardFork::Darwin, ForkCondition::Timestamp(0));
        self
    }

    /// Enable DarwinV2 at genesis
    pub fn darwin_v2_activated(mut self) -> Self {
        self = self.darwin_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardFork::DarwinV2, ForkCondition::Timestamp(0));
        self
    }

    /// Build the resulting [`ScrollChainSpec`].
    ///
    /// # Panics
    ///
    /// This function panics if the chain ID and genesis is not set ([`Self::chain`] and
    /// [`Self::genesis`])
    pub fn build(self) -> ScrollChainSpec {
        ScrollChainSpec { inner: self.inner.build() }
    }
}

/// Scroll chain spec type.
#[derive(Debug, Clone, Deref, Into, Constructor, PartialEq, Eq)]
pub struct ScrollChainSpec {
    /// [`ChainSpec`].
    pub inner: ChainSpec,
}

// // TODO fulfill here when L2 base fee implemented
// impl ScrollChainSpec {
//     /// Read from parent to determine the base fee for the next block
//     pub fn next_block_base_fee(
//         &self,
//         parent: &Header,
//         timestamp: u64,
//     ) -> Result<U256, DecodeError> {
//         Ok(U256::try_from(0).unwrap())
//     }
// }

#[derive(Clone, Debug, Display, Eq, PartialEq)]
/// Error type for decoding Holocene 1559 parameters
pub enum DecodeError {
    #[display("Insufficient data to decode")]
    /// Insufficient data to decode
    InsufficientData,
    #[display("Invalid denominator parameter")]
    /// Invalid denominator parameter
    InvalidDenominator,
    #[display("Invalid elasticity parameter")]
    /// Invalid elasticity parameter
    InvalidElasticity,
}

impl core::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        // None of the errors have sub-errors
        None
    }
}

impl EthChainSpec for ScrollChainSpec {
    fn chain(&self) -> alloy_chains::Chain {
        self.inner.chain()
    }

    fn base_fee_params_at_block(&self, block_number: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_block(block_number)
    }

    fn base_fee_params_at_timestamp(&self, timestamp: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_timestamp(timestamp)
    }

    fn deposit_contract(&self) -> Option<&DepositContract> {
        self.inner.deposit_contract()
    }

    fn genesis_hash(&self) -> B256 {
        self.inner.genesis_hash()
    }

    fn prune_delete_limit(&self) -> usize {
        self.inner.prune_delete_limit()
    }

    fn display_hardforks(&self) -> Box<dyn Display> {
        Box::new(ChainSpec::display_hardforks(self))
    }

    fn genesis_header(&self) -> &Header {
        self.inner.genesis_header()
    }

    fn genesis(&self) -> &Genesis {
        self.inner.genesis()
    }

    fn max_gas_limit(&self) -> u64 {
        self.inner.max_gas_limit()
    }

    fn bootnodes(&self) -> Option<Vec<NodeRecord>> {
        self.inner.bootnodes()
    }
}

impl Hardforks for ScrollChainSpec {
    fn fork<H: reth_chainspec::Hardfork>(&self, fork: H) -> reth_chainspec::ForkCondition {
        self.inner.fork(fork)
    }

    fn forks_iter(
        &self,
    ) -> impl Iterator<Item = (&dyn reth_chainspec::Hardfork, reth_chainspec::ForkCondition)> {
        self.inner.forks_iter()
    }

    fn fork_id(&self, head: &Head) -> ForkId {
        self.inner.fork_id(head)
    }

    fn latest_fork_id(&self) -> ForkId {
        self.inner.latest_fork_id()
    }

    fn fork_filter(&self, head: Head) -> ForkFilter {
        self.inner.fork_filter(head)
    }
}

impl EthereumHardforks for ScrollChainSpec {
    fn get_final_paris_total_difficulty(&self) -> Option<U256> {
        self.inner.get_final_paris_total_difficulty()
    }

    fn final_paris_total_difficulty(&self, block_number: u64) -> Option<U256> {
        self.inner.final_paris_total_difficulty(block_number)
    }
}

impl ScrollHardforks for ScrollChainSpec {}

impl From<Genesis> for ScrollChainSpec {
    fn from(genesis: Genesis) -> Self {
        use reth_scroll_forks::ScrollHardFork;
        let scroll_genesis_info = ScrollGenesisInfo::extract_from(&genesis);
        let genesis_info = scroll_genesis_info.scroll_chain_info.genesis_info.unwrap_or_default();

        // Block-based hardforks
        let hardfork_opts = [
            (EthereumHardfork::Homestead.boxed(), genesis.config.homestead_block),
            (EthereumHardfork::Tangerine.boxed(), genesis.config.eip150_block),
            (EthereumHardfork::SpuriousDragon.boxed(), genesis.config.eip155_block),
            (EthereumHardfork::Byzantium.boxed(), genesis.config.byzantium_block),
            (EthereumHardfork::Constantinople.boxed(), genesis.config.constantinople_block),
            (EthereumHardfork::Petersburg.boxed(), genesis.config.petersburg_block),
            (EthereumHardfork::Istanbul.boxed(), genesis.config.istanbul_block),
            (EthereumHardfork::MuirGlacier.boxed(), genesis.config.muir_glacier_block),
            (EthereumHardfork::Berlin.boxed(), genesis.config.berlin_block),
            (EthereumHardfork::London.boxed(), genesis.config.london_block),
            (EthereumHardfork::ArrowGlacier.boxed(), genesis.config.arrow_glacier_block),
            (EthereumHardfork::GrayGlacier.boxed(), genesis.config.gray_glacier_block),
            (ScrollHardFork::Bernoulli.boxed(), genesis_info.bernoulli_block),
            (ScrollHardFork::Curie.boxed(), genesis_info.curie_block),
        ];
        let mut block_hardforks = hardfork_opts
            .into_iter()
            .filter_map(|(hardfork, opt)| opt.map(|block| (hardfork, ForkCondition::Block(block))))
            .collect::<Vec<_>>();

        // Paris
        let paris_block_and_final_difficulty =
            if let Some(ttd) = genesis.config.terminal_total_difficulty {
                block_hardforks.push((
                    EthereumHardfork::Paris.boxed(),
                    ForkCondition::TTD {
                        total_difficulty: ttd,
                        fork_block: genesis.config.merge_netsplit_block,
                    },
                ));

                genesis.config.merge_netsplit_block.map(|block| (block, ttd))
            } else {
                None
            };

        // Time-based hardforks
        let time_hardfork_opts = [
            (EthereumHardfork::Shanghai.boxed(), genesis.config.shanghai_time),
            (ScrollHardFork::Darwin.boxed(), genesis_info.darwin_time),
            (ScrollHardFork::DarwinV2.boxed(), genesis_info.darwin_v2_time),
        ];

        let mut time_hardforks = time_hardfork_opts
            .into_iter()
            .filter_map(|(hardfork, opt)| {
                opt.map(|time| (hardfork, ForkCondition::Timestamp(time)))
            })
            .collect::<Vec<_>>();

        block_hardforks.append(&mut time_hardforks);

        // Ordered Hardforks
        let mainnet_hardforks = ScrollHardFork::scroll_mainnet();
        let mainnet_order = mainnet_hardforks.forks_iter();

        let mut ordered_hardforks = Vec::with_capacity(block_hardforks.len());
        for (hardfork, _) in mainnet_order {
            if let Some(pos) = block_hardforks.iter().position(|(e, _)| **e == *hardfork) {
                ordered_hardforks.push(block_hardforks.remove(pos));
            }
        }

        // append the remaining unknown hardforks to ensure we don't filter any out
        ordered_hardforks.append(&mut block_hardforks);

        Self {
            inner: ChainSpec {
                chain: genesis.config.chain_id.into(),
                genesis,
                hardforks: ChainHardforks::new(ordered_hardforks),
                paris_block_and_final_difficulty,
                ..Default::default()
            },
        }
    }
}

#[derive(Default, Debug)]
struct ScrollGenesisInfo {
    scroll_chain_info: ScrollChainInfo,
}

impl ScrollGenesisInfo {
    fn extract_from(genesis: &Genesis) -> Self {
        let info = Self {
            scroll_chain_info: ScrollChainInfo::extract_from(&genesis.config.extra_fields)
                .unwrap_or_default(),
            ..Default::default()
        };
        info
    }
}
