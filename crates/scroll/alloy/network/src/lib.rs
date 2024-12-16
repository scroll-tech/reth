#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub use alloy_network::*;

use scroll_alloy_consensus::{self, ScrollTxType};
use scroll_alloy_rpc_types;

/// Types for an Op-stack network.
#[derive(Clone, Copy, Debug)]
pub struct Scroll {
    _private: (),
}

impl Network for Scroll {
    type TxType = ScrollTxType;

    type TxEnvelope = scroll_alloy_consensus::ScrollTxEnvelope;

    type UnsignedTx = scroll_alloy_consensus::ScrollTypedTransaction;

    type ReceiptEnvelope = scroll_alloy_consensus::ScrollTxEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = scroll_alloy_rpc_types::ScrollTransactionRequest;

    type TransactionResponse = scroll_alloy_rpc_types::Transaction;

    type ReceiptResponse = scroll_alloy_rpc_types::ScrollTransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::Header;

    type BlockResponse =
        alloy_rpc_types_eth::Block<Self::TransactionResponse, Self::HeaderResponse>;
}
