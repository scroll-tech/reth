//! Chain specification for the Scroll Sepolia testnet network.

use alloc::sync::Arc;

use alloy_chains::Chain;
use alloy_primitives::b256;
use reth_chainspec::{once_cell_set, ChainSpec};
use reth_scroll_forks::ScrollHardfork;

use crate::{LazyLock, ScrollChainSpec};

/// The Scroll Sepolia spec
pub static SCROLL_SEPOLIA: LazyLock<Arc<ScrollChainSpec>> = LazyLock::new(|| {
    ScrollChainSpec {
        inner: ChainSpec {
            chain: Chain::scroll_sepolia(),
            genesis: serde_json::from_str(include_str!("../res/genesis/sepolia_scroll.json"))
                .expect("Can't deserialize Scroll Sepolia genesis json"),
            genesis_hash: once_cell_set(b256!(
                "aa62d1a8b2bffa9e5d2368b63aae0d98d54928bd713125e3fd9e5c896c68592c"
            )),
            hardforks: ScrollHardfork::scroll_sepolia(),
            ..Default::default()
        },
    }
    .into()
});
