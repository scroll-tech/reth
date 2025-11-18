//! Chain specification for the Scroll Sepolia testnet network.

use crate::{
    constants::SCROLL_BASE_FEE_PARAMS_FEYNMAN, make_genesis_header, LazyLock, ScrollChainConfig,
    ScrollChainSpec, SCROLL_SEPOLIA_GENESIS_HASH,
};
use alloc::{sync::Arc, vec};

use alloy_chains::Chain;
use reth_chainspec::{BaseFeeParamsKind, ChainSpec, Hardfork};
use reth_primitives_traits::SealedHeader;
use reth_scroll_forks::SCROLL_SEPOLIA_HARDFORKS;
use scroll_alloy_hardforks::ScrollHardfork;

/// The Scroll Sepolia spec
pub static SCROLL_SEPOLIA: LazyLock<Arc<ScrollChainSpec>> = LazyLock::new(|| {
    let genesis = serde_json::from_str(include_str!("../res/genesis/sepolia_scroll.json"))
        .expect("Can't deserialize Scroll Sepolia genesis json");
    ScrollChainSpec {
        inner: ChainSpec {
            chain: Chain::scroll_sepolia(),
            genesis_header: SealedHeader::new(
                make_genesis_header(&genesis),
                SCROLL_SEPOLIA_GENESIS_HASH,
            ),
            genesis,
            hardforks: SCROLL_SEPOLIA_HARDFORKS.clone(),
            base_fee_params: BaseFeeParamsKind::Variable(
                vec![(ScrollHardfork::Feynman.boxed(), SCROLL_BASE_FEE_PARAMS_FEYNMAN)].into(),
            ),
            ..Default::default()
        },
        config: ScrollChainConfig::sepolia(),
    }
    .into()
});
