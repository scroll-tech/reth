//! RPC errors specific to Scroll.

use alloy_rpc_types_eth::{error::EthRpcErrorCode, BlockError};
use jsonrpsee_types::error::INTERNAL_ERROR_CODE;
use reth_optimism_evm::OpBlockExecutionError;
use reth_rpc_eth_api::AsEthApiError;
use reth_rpc_eth_types::EthApiError;
use reth_rpc_server_types::result::{internal_rpc_err, rpc_err};
use revm::primitives::{InvalidTransaction, OptimismInvalidTransaction};

/// Scroll specific errors, that extend [`EthApiError`].
#[derive(Debug, thiserror::Error)]
pub enum ScrollEthApiError {
    /// L1 ethereum error.
    #[error(transparent)]
    Eth(#[from] EthApiError),
    /// EVM error originating from invalid Scroll data.
    #[error(transparent)]
    Evm(#[from] ScrollBlockExecutionError),
    /// Thrown when calculating L1 gas fee.
    #[error("failed to calculate l1 gas fee")]
    L1BlockFeeError,
    /// Thrown when calculating L1 gas used
    #[error("failed to calculate l1 gas used")]
    L1BlockGasError,
    /// Wrapper for [`revm_primitives::InvalidTransaction`](InvalidTransaction).
    #[error(transparent)]
    InvalidTransaction(#[from] ScrollInvalidTransactionError),
    /// Sequencer client error.
    #[error(transparent)]
    Sequencer(#[from] SequencerClientError),
}

impl AsEthApiError for ScrollEthApiError {
    fn as_err(&self) -> Option<&EthApiError> {
        match self {
            Self::Eth(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ScrollEthApiError> for jsonrpsee_types::error::ErrorObject<'static> {
    fn from(err: ScrollEthApiError) -> Self {
        match err {
            ScrollEthApiError::Eth(err) => err.into(),
            ScrollEthApiError::InvalidTransaction(err) => err.into(),
            ScrollEthApiError::Evm(_) |
            ScrollEthApiError::L1BlockFeeError |
            ScrollEthApiError::L1BlockGasError => internal_rpc_err(err.to_string()),
            ScrollEthApiError::Sequencer(err) => err.into(),
        }
    }
}

/// Scroll specific invalid transaction errors
#[derive(thiserror::Error, Debug)]
pub enum ScrollInvalidTransactionError {}

impl From<ScrollInvalidTransactionError> for jsonrpsee_types::error::ErrorObject<'static> {
    fn from(err: ScrollInvalidTransactionError) -> Self {
        match err {}
    }
}

impl TryFrom<InvalidTransaction> for ScrollInvalidTransactionError {
    type Error = InvalidTransaction;

    fn try_from(err: InvalidTransaction) -> Result<Self, Self::Error> {
        match err {
            _ => Err(err),
        }
    }
}

/// Error type when interacting with the Sequencer
#[derive(Debug, thiserror::Error)]
pub enum SequencerClientError {
    /// Wrapper around an [`reqwest::Error`].
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    /// Thrown when serializing transaction to forward to sequencer
    #[error("invalid sequencer transaction")]
    InvalidSequencerTransaction,
}

impl From<SequencerClientError> for jsonrpsee_types::error::ErrorObject<'static> {
    fn from(err: SequencerClientError) -> Self {
        jsonrpsee_types::error::ErrorObject::owned(
            INTERNAL_ERROR_CODE,
            err.to_string(),
            None::<String>,
        )
    }
}

impl From<BlockError> for ScrollEthApiError {
    fn from(error: BlockError) -> Self {
        Self::Eth(error.into())
    }
}
