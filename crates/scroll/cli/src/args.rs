use reth_node_builder::engine_tree_config::{
    DEFAULT_MEMORY_BLOCK_BUFFER_TARGET, DEFAULT_PERSISTENCE_THRESHOLD,
};

#[derive(Debug, clap::Args)]
pub struct ScrollRollupArgs {
    /// Configure persistence threshold for engine experimental.
    #[arg(long = "engine.persistence-threshold", conflicts_with = "legacy", default_value_t = DEFAULT_PERSISTENCE_THRESHOLD)]
    pub persistence_threshold: u64,

    /// Configure the target number of blocks to keep in memory.
    #[arg(long = "engine.memory-block-buffer-target", conflicts_with = "legacy", default_value_t = DEFAULT_MEMORY_BLOCK_BUFFER_TARGET)]
    pub memory_block_buffer_target: u64,
}
