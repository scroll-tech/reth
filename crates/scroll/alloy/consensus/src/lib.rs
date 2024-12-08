#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
extern crate alloc;

mod transaction;
pub use transaction::{
    ScrollTxEnvelope, ScrollTxType, ScrollTypedTransaction, TxL1Message, L1_MESSAGE_TX_TYPE_ID,
};

#[cfg(feature = "serde")]
pub use transaction::serde_l1_message_tx_rpc;
