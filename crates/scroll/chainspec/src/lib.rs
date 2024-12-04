//! Scroll-Reth chain specs.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod dev;
mod genesis;
mod scroll;
mod scroll_sepolia;

use alloc::{boxed::Box, vec::Vec};
use alloy_chains::Chain;
use alloy_consensus::Header;
use alloy_genesis::Genesis;
use alloy_primitives::{B256, U256};
use derive_more::{Constructor, Deref, Display, From, Into};
pub use dev::SCROLL_DEV;
pub use genesis::ScrollChainInfo;
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
    pub fn without_fork(mut self, fork: reth_scroll_forks::ScrollHardfork) -> Self {
        self.inner = self.inner.without_fork(fork);
        self
    }

    /// Enable Bernoulli at genesis
    pub fn bernoulli_activated(mut self) -> Self {
        self.inner = self.inner.cancun_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardfork::Bernoulli, ForkCondition::Block(0));
        self
    }

    /// Enable Curie at genesis
    pub fn curie_activated(mut self) -> Self {
        self = self.bernoulli_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardfork::Curie, ForkCondition::Timestamp(0));
        self
    }

    /// Enable Darwin at genesis
    pub fn darwin_activated(mut self) -> Self {
        self = self.curie_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardfork::Darwin, ForkCondition::Timestamp(0));
        self
    }

    /// Enable `DarwinV2` at genesis
    pub fn darwin_v2_activated(mut self) -> Self {
        self = self.darwin_activated();
        self.inner = self
            .inner
            .with_fork(reth_scroll_forks::ScrollHardfork::DarwinV2, ForkCondition::Timestamp(0));
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
        use reth_scroll_forks::ScrollHardfork;
        let scroll_genesis_info = ScrollGenesisInfo::extract_from(&genesis);
        let genesis_info =
            scroll_genesis_info.scroll_chain_info.genesis_info.expect("load scroll genesis info");

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
            (ScrollHardfork::Bernoulli.boxed(), genesis_info.bernoulli_block),
            (ScrollHardfork::Curie.boxed(), genesis_info.curie_block),
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
            (ScrollHardfork::Darwin.boxed(), genesis_info.darwin_time),
            (ScrollHardfork::DarwinV2.boxed(), genesis_info.darwin_v2_time),
        ];

        let mut time_hardforks = time_hardfork_opts
            .into_iter()
            .filter_map(|(hardfork, opt)| {
                opt.map(|time| (hardfork, ForkCondition::Timestamp(time)))
            })
            .collect::<Vec<_>>();

        block_hardforks.append(&mut time_hardforks);

        // Ordered Hardforks
        let mainnet_hardforks = ScrollHardfork::scroll_mainnet();
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
        Self {
            scroll_chain_info: ScrollChainInfo::extract_from(&genesis.config.extra_fields)
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use alloy_genesis::{ChainConfig, Genesis};
    use reth_ethereum_forks::{EthereumHardfork, ForkHash, ForkId};
    use reth_scroll_forks::ScrollHardfork;

    #[test]
    // fn scroll_mainnet_forkids() {
    //     let scroll_mainnet = ScrollChainSpecBuilder::scroll_mainnet().build();
    //     let _ =
    //         scroll_mainnet.genesis_hash.set(SCROLL_MAINNET.genesis_hash.get().copied().unwrap());
    //     test_fork_ids(
    //         &SCROLL_MAINNET,
    //         &[
    //             (
    //                 Head { number: 0, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x67, 0xda, 0x02, 0x60]), next: 1704992401 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1704992400, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x67, 0xda, 0x02, 0x60]), next: 1704992401 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1704992401, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x3c, 0x28, 0x3c, 0xb3]), next: 1710374401 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1710374400, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x3c, 0x28, 0x3c, 0xb3]), next: 1710374401 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1710374401, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x51, 0xcc, 0x98, 0xb3]), next: 1720627201 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1720627200, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x51, 0xcc, 0x98, 0xb3]), next: 1720627201 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1720627201, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xe4, 0x01, 0x0e, 0xb9]), next: 1726070401 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1726070401, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xbc, 0x38, 0xf9, 0xca]), next: 0 },
    //             ),
    //         ],
    //     );
    // }
    //
    // #[test]
    // fn scroll_sepolia_forkids() {
    //     test_fork_ids(
    //         &SCROLL_SEPOLIA,
    //         &[
    //             (
    //                 Head { number: 0, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x67, 0xa4, 0x03, 0x28]), next: 1699981200 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1699981199, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x67, 0xa4, 0x03, 0x28]), next: 1699981200 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1699981200, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xa4, 0x8d, 0x6a, 0x00]), next: 1708534800 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1708534799, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xa4, 0x8d, 0x6a, 0x00]), next: 1708534800 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1708534800, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xcc, 0x17, 0xc7, 0xeb]), next: 1716998400 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1716998399, ..Default::default() },
    //                 ForkId { hash: ForkHash([0xcc, 0x17, 0xc7, 0xeb]), next: 1716998400 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1716998400, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x54, 0x0a, 0x8c, 0x5d]), next: 1723478400 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1723478399, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x54, 0x0a, 0x8c, 0x5d]), next: 1723478400 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1723478400, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x75, 0xde, 0xa4, 0x1e]), next: 1732633200 },
    //             ),
    //             (
    //                 Head { number: 0, timestamp: 1732633200, ..Default::default() },
    //                 ForkId { hash: ForkHash([0x4a, 0x1c, 0x79, 0x2e]), next: 0 },
    //             ),
    //         ],
    //     );
    // }

    // #[test]
    // fn scroll_mainnet_genesis() {
    //     let genesis = SCROLL_MAINNET.genesis_header();
    //     println!("{:?}", genesis);
    //     assert_eq!(
    //         genesis.hash_slow(),
    //         b256!("bbc05efd412b7cd47a2ed0e5ddfcf87af251e414ea4c801d78b6784513180a80")
    //     );
    //     let base_fee = genesis
    //         .next_block_base_fee(SCROLL_MAINNET.base_fee_params_at_timestamp(genesis.timestamp))
    //         .unwrap();
    //     // <https://scrollscan.com/block/1>
    //     assert_eq!(base_fee, 980000000);
    // }

    // #[test]
    // fn scroll_sepolia_genesis() {
    //     let genesis = SCROLL_SEPOLIA.genesis_header();
    //     assert_eq!(
    //         genesis.hash_slow(),
    //         b256!("aa62d1a8b2bffa9e5d2368b63aae0d98d54928bd713125e3fd9e5c896c68592c")
    //     );
    //     let base_fee = genesis
    //         .next_block_base_fee(SCROLL_SEPOLIA.base_fee_params_at_timestamp(genesis.timestamp))
    //         .unwrap();
    //     // <https://base-sepolia.blockscout.com/block/1>
    //     assert_eq!(base_fee, 980000000);
    // }
    #[test]
    fn latest_scroll_mainnet_fork_id() {
        let a = SCROLL_MAINNET.latest_fork_id();
        assert_eq!(
            ForkId { hash: ForkHash([0xbc, 0x38, 0xf9, 0xca]), next: 0 },
            SCROLL_MAINNET.latest_fork_id()
        )
    }

    #[test]
    fn latest_scroll_mainnet_fork_id_with_builder() {
        let scroll_mainnet = ScrollChainSpecBuilder::scroll_mainnet().build();
        let a = scroll_mainnet.latest_fork_id();
        assert_eq!(
            ForkId { hash: ForkHash([0xbc, 0x38, 0xf9, 0xca]), next: 0 },
            scroll_mainnet.latest_fork_id()
        )
    }

    #[test]
    fn is_bernoulli_active() {
        let scroll_mainnet = ScrollChainSpecBuilder::scroll_mainnet().build();
        assert!(!scroll_mainnet.is_bernoulli_active_at_block(1))
    }

    #[test]
    fn parse_scroll_hardforks() {
        let geth_genesis = r#"
    {
      "config": {
        "bernoulliBlock": 10,
        "curieBlock": 20,
        "darwinTime": 30,
        "darwinV2Time": 31,
        "scroll": {
            "feeVaultAddress": "0x5300000000000000000000000000000000000005",
            "l1Config": {
                "l1ChainId": "1",
                "l1MessageQueueAddress": "0x0d7E906BD9cAFa154b048cFa766Cc1E54E39AF9B",
                "scrollChainAddress": "0xa13BAF47339d63B743e7Da8741db5456DAc1E556",
                "numL1MessagesPerBlock": "10"
            }
        }
      }
    }
    "#;
        let genesis: Genesis = serde_json::from_str(geth_genesis).unwrap();

        let actual_bernoulli_block = genesis.config.extra_fields.get("bernoulliBlock");
        assert_eq!(actual_bernoulli_block, Some(serde_json::Value::from(10)).as_ref());
        let actual_curie_block = genesis.config.extra_fields.get("curieBlock");
        assert_eq!(actual_curie_block, Some(serde_json::Value::from(20)).as_ref());
        let actual_darwin_timestamp = genesis.config.extra_fields.get("darwinTime");
        assert_eq!(actual_darwin_timestamp, Some(serde_json::Value::from(30)).as_ref());
        let actual_darwin_v2_timestamp = genesis.config.extra_fields.get("darwinV2Time");
        assert_eq!(actual_darwin_v2_timestamp, Some(serde_json::Value::from(31)).as_ref());
        let scroll_object = genesis.config.extra_fields.get("scroll").unwrap();
        assert_eq!(
            scroll_object,
            &serde_json::json!({
                "feeVaultAddress": "0x5300000000000000000000000000000000000005",
                "l1Config": {
                    "l1ChainId": "1",
                    "l1MessageQueueAddress": "0x0d7E906BD9cAFa154b048cFa766Cc1E54E39AF9B",
                    "scrollChainAddress": "0xa13BAF47339d63B743e7Da8741db5456DAc1E556",
                    "numL1MessagesPerBlock": "10"
                }
            })
        );

        let chain_spec: ScrollChainSpec = genesis.into();

        assert!(!chain_spec.is_fork_active_at_block(ScrollHardfork::Bernoulli, 0));
        assert!(!chain_spec.is_fork_active_at_block(ScrollHardfork::Curie, 0));
        assert!(!chain_spec.is_fork_active_at_timestamp(ScrollHardfork::Darwin, 0));
        assert!(!chain_spec.is_fork_active_at_timestamp(ScrollHardfork::DarwinV2, 0));

        assert!(chain_spec.is_fork_active_at_block(ScrollHardfork::Bernoulli, 10));
        assert!(chain_spec.is_fork_active_at_block(ScrollHardfork::Curie, 20));
        assert!(chain_spec.is_fork_active_at_timestamp(ScrollHardfork::Darwin, 30));
        assert!(chain_spec.is_fork_active_at_timestamp(ScrollHardfork::DarwinV2, 31));
    }

    #[test]
    fn test_fork_order_scroll_mainnet() {
        let genesis = Genesis {
            config: ChainConfig {
                chain_id: 0,
                homestead_block: Some(0),
                dao_fork_block: Some(0),
                dao_fork_support: false,
                eip150_block: Some(0),
                eip155_block: Some(0),
                eip158_block: Some(0),
                byzantium_block: Some(0),
                constantinople_block: Some(0),
                petersburg_block: Some(0),
                istanbul_block: Some(0),
                muir_glacier_block: Some(0),
                berlin_block: Some(0),
                london_block: Some(0),
                arrow_glacier_block: Some(0),
                gray_glacier_block: Some(0),
                merge_netsplit_block: Some(0),
                shanghai_time: Some(0),
                terminal_total_difficulty: Some(U256::ZERO),
                extra_fields: [
                    (String::from("bernoulliBlock"), 0.into()),
                    (String::from("curieBlock"), 0.into()),
                    (String::from("darwinTime"), 0.into()),
                    (String::from("darwinV2Time"), 0.into()),
                ]
                .into_iter()
                .collect(),
                ..Default::default()
            },
            ..Default::default()
        };

        let chain_spec: ScrollChainSpec = genesis.into();

        let hardforks: Vec<_> = chain_spec.hardforks.forks_iter().map(|(h, _)| h).collect();
        let expected_hardforks = vec![
            EthereumHardfork::Homestead.boxed(),
            EthereumHardfork::Tangerine.boxed(),
            EthereumHardfork::SpuriousDragon.boxed(),
            EthereumHardfork::Byzantium.boxed(),
            EthereumHardfork::Constantinople.boxed(),
            EthereumHardfork::Petersburg.boxed(),
            EthereumHardfork::Istanbul.boxed(),
            EthereumHardfork::MuirGlacier.boxed(),
            EthereumHardfork::Berlin.boxed(),
            EthereumHardfork::London.boxed(),
            EthereumHardfork::ArrowGlacier.boxed(),
            EthereumHardfork::GrayGlacier.boxed(),
            EthereumHardfork::Paris.boxed(),
            EthereumHardfork::Shanghai.boxed(),
            ScrollHardfork::Bernoulli.boxed(),
            ScrollHardfork::Curie.boxed(),
            ScrollHardfork::Darwin.boxed(),
            ScrollHardfork::DarwinV2.boxed(),
        ];

        assert!(expected_hardforks
            .iter()
            .zip(hardforks.iter())
            .all(|(expected, actual)| &**expected == *actual));
        assert_eq!(expected_hardforks.len(), hardforks.len());
    }
}
