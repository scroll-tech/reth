#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub use alloy_network::*;

use alloy_consensus::{TxEnvelope, TxType, TypedTransaction};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, U256};
use alloy_rpc_types_eth::AccessList;
use scroll_alloy_consensus::{ScrollTxEnvelope, ScrollTxType, ScrollTypedTransaction};
use scroll_alloy_rpc_types::OpTransactionRequest;

/// Types for an Op-stack network.
#[derive(Clone, Copy, Debug)]
pub struct Optimism {
    _private: (),
}

impl Network for Optimism {
    type TxType = ScrollTxType;

    type TxEnvelope = scroll_alloy_consensus::ScrollTxEnvelope;

    type UnsignedTx = scroll_alloy_consensus::ScrollTypedTransaction;

    type ReceiptEnvelope = scroll_alloy_consensus::OpReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = scroll_alloy_rpc_types::OpTransactionRequest;

    type TransactionResponse = scroll_alloy_rpc_types::Transaction;

    type ReceiptResponse = scroll_alloy_rpc_types::OpTransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::Header;

    type BlockResponse =
        alloy_rpc_types_eth::Block<Self::TransactionResponse, Self::HeaderResponse>;
}
