use crate::builder::payload::SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT;

use reth_scroll_rpc::eth::DEFAULT_MIN_SUGGESTED_PRIORITY_FEE;

/// Rollup arguments for the Scroll node.
#[derive(Debug, Clone, clap::Args)]
pub struct ScrollRollupArgs {
    /// Endpoint for the sequencer mempool (can be both HTTP and WS)
    #[arg(long = "scroll.sequencer")]
    pub sequencer: Option<String>,

    /// Minimum suggested priority fee (tip) in wei, default to
    /// [`DEFAULT_MIN_SUGGESTED_PRIORITY_FEE`].
    #[arg(long = "scroll.min-suggested-priority-fee", default_value_t = DEFAULT_MIN_SUGGESTED_PRIORITY_FEE)]
    pub min_suggested_priority_fee: u64,

    /// Payload size limit, default to `122kB`.
    #[arg(long = "scroll.payload-size-limit", default_value_t = SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT)]
    pub payload_size_limit: u64,
}

impl Default for ScrollRollupArgs {
    fn default() -> Self {
        Self {
            sequencer: None,
            min_suggested_priority_fee: DEFAULT_MIN_SUGGESTED_PRIORITY_FEE,
            payload_size_limit: SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT,
        }
    }
}
