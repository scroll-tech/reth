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
    #[error("Sequencer transactions over gas limit: {gas}; gas spent by each transaction: {gas_spent_by_tx:?}")]
    BlockGasLimitExceededBySequencerTransactions {
        /// The gas used by each transaction in the block.
        gas_spent_by_tx: Vec<u64>,
        /// The block gas limit.
        gas: u64,
    },
}
