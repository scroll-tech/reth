use derive_more::{Display, From};
use reth_consensus::ConsensusError;
use reth_evm::execute::{BlockExecutionError, BlockValidationError, ProviderError};

/// Execution error for Scroll.
#[derive(thiserror::Error, Display, From, Debug)]
pub enum ScrollBlockExecutionError {
    /// Error occurred at block execution.
    BlockExecution(BlockExecutionError),
    /// Error occurred at a hard fork.
    #[display("failed to apply hard fork: {_0}")]
    HardFork(HardForkError),
    /// Error occurred at L1 fee computation.
    #[display("failed to compute l1 fee: {reason}")]
    L1FeeComputation {
        /// The reason for the fee computation error.
        reason: &'static str,
    },
}

impl ScrollBlockExecutionError {
    /// Returns a [`ScrollBlockExecutionError`] with the `L1FeeComputation` variant.
    pub const fn l1_fee(reason: &'static str) -> Self {
        Self::L1FeeComputation { reason }
    }
}

/// Scroll hard fork error.
#[derive(Debug, Display)]
pub enum HardForkError {
    /// Error occurred at the Curie hard fork.
    Curie,
}

impl From<ProviderError> for ScrollBlockExecutionError {
    fn from(value: ProviderError) -> Self {
        Self::BlockExecution(value.into())
    }
}

impl From<BlockValidationError> for ScrollBlockExecutionError {
    fn from(value: BlockValidationError) -> Self {
        Self::BlockExecution(value.into())
    }
}

impl From<ConsensusError> for ScrollBlockExecutionError {
    fn from(value: ConsensusError) -> Self {
        Self::BlockExecution(value.into())
    }
}
