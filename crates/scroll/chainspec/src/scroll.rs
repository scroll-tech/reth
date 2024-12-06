//! Chain specification for the Scroll Mainnet network.

use alloc::sync::Arc;

use alloy_chains::Chain;
use alloy_primitives::b256;
use reth_chainspec::{once_cell_set, ChainSpec};
use reth_scroll_forks::ScrollHardfork;

use crate::{LazyLock, ScrollChainSpec};

/// The Scroll Mainnet spec
pub static SCROLL_MAINNET: LazyLock<Arc<ScrollChainSpec>> = LazyLock::new(|| {
    ScrollChainSpec {
        inner: ChainSpec {
            chain: Chain::scroll_mainnet(),
            // genesis contains empty alloc field because state at first bedrock block is imported
            // manually from trusted source
            genesis: serde_json::from_str(include_str!("../res/genesis/scroll.json"))
                .expect("Can't deserialize Scroll Mainnet genesis json"),
            genesis_hash: once_cell_set(b256!(
                "bbc05efd412b7cd47a2ed0e5ddfcf87af251e414ea4c801d78b6784513180a80"
            )),
            hardforks: ScrollHardfork::scroll_mainnet(),
            ..Default::default()
        },
    }
    .into()
});
