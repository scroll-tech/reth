use derive_more::{Display, From};
use reth_evm::execute::BlockExecutionError;

/// Execution error for Scroll.
#[derive(thiserror::Error, Display, From, Debug)]
pub enum ScrollBlockExecutionError {
    /// Error occurred at a hard fork.
    #[display("failed to apply fork: {_0}")]
    Fork(ForkError),
    /// Error occurred at L1 fee computation.
    #[display("failed to compute l1 fee: {reason}")]
    L1FeeComputation {
        /// The reason for the fee computation error.
        reason: &'static str,
    },
}

impl From<ScrollBlockExecutionError> for BlockExecutionError {
    fn from(value: ScrollBlockExecutionError) -> Self {
        Self::other(value)
    }
}

impl ScrollBlockExecutionError {
    /// Returns a [`ScrollBlockExecutionError`] with the `L1FeeComputation` variant.
    pub const fn l1_fee(reason: &'static str) -> Self {
        Self::L1FeeComputation { reason }
    }
}

/// Scroll fork error.
#[derive(Debug, Display)]
pub enum ForkError {
    /// Error occurred at the Curie hard fork.
    Curie,
}

impl From<ForkError> for BlockExecutionError {
    fn from(value: ForkError) -> Self {
        ScrollBlockExecutionError::Fork(value).into()
    }
}
