//! Transaction types for Scroll.

mod tx_type;
pub use tx_type::{ScrollTxType, L1_MESSAGE_TX_TYPE_ID};

mod envelope;
pub use envelope::ScrollTxEnvelope;

mod l1_message;
pub use l1_message::TxL1Message;

mod typed;
pub use typed::ScrollTypedTransaction;

#[cfg(feature = "serde")]
pub use l1_message::serde_l1_message_tx_rpc;

/// Bincode-compatible serde implementations for transaction types.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(super) mod serde_bincode_compat {
    pub use super::l1_message::serde_bincode_compat::TxL1Message;
}
