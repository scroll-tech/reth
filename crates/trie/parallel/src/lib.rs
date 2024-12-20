//! Implementation of exotic state root computation approaches.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod storage_root_targets;
pub use storage_root_targets::StorageRootTargets;

/// Parallel database state root.
mod db;
pub use db::ParallelDatabaseStateRoot;

/// Parallel state commitment.
mod commitment;
pub use commitment::ParallelStateCommitment;

/// Parallel trie calculation stats.
pub mod stats;

/// Implementation of parallel state root computation.
pub mod root;

/// Implementation of parallel proof computation.
pub mod proof;

/// Parallel state root metrics.
#[cfg(feature = "metrics")]
pub mod metrics;
