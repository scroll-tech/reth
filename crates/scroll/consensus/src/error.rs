use crate::constants::SCROLL_MAXIMUM_BASE_FEE;

use alloy_primitives::{Address, B256, B64, U256};
use reth_consensus::ConsensusError;

/// Scroll consensus error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ScrollConsensusError {
    /// L1 [`ConsensusError`], that also occurs on L2.
    #[error(transparent)]
    Eth(#[from] ConsensusError),
    /// Invalid L1 messages order.
    #[error("invalid L1 message order")]
    InvalidL1MessageOrder,
    /// Block has non zero coinbase.
    #[error("block coinbase not zero: {0}")]
    CoinbaseNotZero(Address),
    /// Block has non zero nonce.
    #[error("block nonce not zero: {0:?}")]
    NonceNotZero(Option<B64>),
    /// Block has invalid clique nonce.
    #[error("block nonce should be 0x0 or 0xffffffffffffffff: {0:?}")]
    InvalidCliqueNonce(Option<B64>),
    /// Block has non zero mix hash.
    #[error("block mix hash not zero: {0:?}")]
    MixHashNotZero(Option<B256>),
    /// Block difficulty is not one.
    #[error("block difficulty not one: {0}")]
    DifficultyNotOne(U256),
    /// Block has invalid clique difficulty.
    #[error("block difficulty should be 1 or 2: {0}")]
    InvalidCliqueDifficulty(U256),
    /// Block extra data missing vanity.
    #[error("block extra data missing vanity")]
    MissingVanity,
    /// Block extra data missing signature.
    #[error("block extra data missing signature")]
    MissingSignature,
    /// Block extra data with invalid checkpoint signers.
    #[error("block extra data contains invalid checkpoint signers")]
    InvalidCheckpointSigners,
    /// Block base fee present before Curie.
    #[error("block base fee is set before Curie fork activation")]
    UnexpectedBaseFee,
    /// Block base fee over limit.
    #[error("block base fee is over limit of {SCROLL_MAXIMUM_BASE_FEE}")]
    BaseFeeOverLimit,
    /// Block body has non-empty withdrawals list.
    #[error("non-empty block body withdrawals list")]
    WithdrawalsNonEmpty,
    /// Chain spec yielded unexpected blob params.
    #[error("unexpected blob params at timestamp")]
    UnexpectedBlobParams,
}

impl From<ScrollConsensusError> for ConsensusError {
    fn from(value: ScrollConsensusError) -> Self {
        match value {
            ScrollConsensusError::Eth(eth) => eth,
            err => Self::Other(err.to_string()),
        }
    }
}
