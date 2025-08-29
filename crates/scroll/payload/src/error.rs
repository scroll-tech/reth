/// Scroll specific payload building errors.
#[derive(Debug, thiserror::Error)]
pub enum ScrollPayloadBuilderError {
    /// Thrown when a transaction fails to convert to a
    /// [`alloy_consensus::transaction::Recovered`].
    #[error("failed to convert deposit transaction to RecoveredTx")]
    TransactionEcRecoverFailed,
    /// Thrown when a blob transaction is included in a sequencer's block.
    #[error("blob transaction included in sequencer block")]
    BlobTransactionRejected,
    /// Thrown when sequencer transaction gas limit exceeds remaining block gas.
    #[error(
        "Sequencer transaction gas limit {gas_limit} exceeds remaining block gas {remaining_gas}, cannot skip sequencer transactions"
    )]
    L1MessageGasExceedsBlock {
        /// The gas limit of the sequencer transaction
        gas_limit: u64,
        /// The remaining gas available in the block
        remaining_gas: u64,
    },
}
