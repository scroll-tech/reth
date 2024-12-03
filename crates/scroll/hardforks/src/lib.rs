//! Scroll-Reth hard forks.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

extern crate alloc;

pub mod hardfork;

mod dev;

pub use dev::DEV_HARDFORKS;
pub use hardfork::ScrollHardfork;

use reth_ethereum_forks::EthereumHardforks;

/// Extends [`EthereumHardforks`] with scroll helper methods.
pub trait ScrollHardforks: EthereumHardforks {
    /// Convenience method to check if [`Bernoulli`](ScrollHardfork::Bernoulli) is active at a given
    /// block number.
    fn is_bernoulli_active_at_block(&self, block_number: u64) -> bool {
        self.fork(ScrollHardfork::Bernoulli).active_at_block(block_number)
    }

    /// Returns `true` if [`Curie`](ScrollHardfork::Curie) is active at given block block number.
    fn is_curie_active_at_block(&self, block_number: u64) -> bool {
        self.fork(ScrollHardfork::Curie).active_at_block(block_number)
    }

    /// Returns `true` if [`Darwin`](ScrollHardfork::Darwin) is active at given block timestamp.
    fn is_darwin_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.fork(ScrollHardfork::Darwin).active_at_timestamp(timestamp)
    }

    /// Returns `true` if [`DarwinV2`](ScrollHardfork::DarwinV2) is active at given block timestamp.
    fn is_darwin_v2_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.fork(ScrollHardfork::DarwinV2).active_at_timestamp(timestamp)
    }
}
