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
pub use hardfork::ScrollHardFork;

use reth_ethereum_forks::EthereumHardforks;

/// Extends [`EthereumHardforks`] with scroll helper methods.
pub trait ScrollHardforks: EthereumHardforks {
    /// Convenience method to check if [`Bernoulli`](ScrollHardFork::Bernoulli) is active at a given
    /// block number.
    fn is_bernoulli_active_at_block(&self, block_number: u64) -> bool {
        self.fork(ScrollHardFork::Bernoulli).active_at_block(block_number)
    }

    /// Returns `true` if [`Curie`](ScrollHardFork::Curie) is active at given block block number.
    fn is_curie_active_at_block(&self, block_number: u64) -> bool {
        self.fork(ScrollHardFork::Curie).active_at_block(block_number)
    }

    /// Returns `true` if [`Darwin`](ScrollHardFork::Darwin) is active at given block timestamp.
    fn is_darwin_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.fork(ScrollHardFork::Darwin).active_at_timestamp(timestamp)
    }

    /// Returns `true` if [`DarwinV2`](ScrollHardFork::DarwinV2) is active at given block timestamp.
    fn is_darwin_v2_active_at_timestamp(&self, timestamp: u64) -> bool {
        self.fork(ScrollHardFork::DarwinV2).active_at_timestamp(timestamp)
    }
}
