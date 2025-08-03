//! Loads and formats Scroll receipt RPC response.

use crate::{ScrollEthApi, ScrollEthApiError};
use alloy_rpc_types_eth::{Log, TransactionReceipt};
use reth_primitives_traits::NodePrimitives;
use reth_rpc_convert::{
    transaction::{ConvertReceiptInput, ReceiptConverter},
    RpcConvert,
};
use reth_rpc_eth_api::{helpers::LoadReceipt, RpcNodeCore};
use reth_rpc_eth_types::receipt::build_receipt;
use reth_scroll_primitives::{ScrollReceipt, ScrollTransactionSigned};
use scroll_alloy_consensus::ScrollReceiptEnvelope;
use scroll_alloy_rpc_types::{ScrollTransactionReceipt, ScrollTransactionReceiptFields};
use std::fmt::Debug;

impl<N, Rpc> LoadReceipt for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}

/// Converter for Scroll receipts.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ScrollReceiptConverter;

impl<N> ReceiptConverter<N> for ScrollReceiptConverter
where
    N: NodePrimitives<SignedTx = ScrollTransactionSigned, Receipt = ScrollReceipt>,
{
    type RpcReceipt = ScrollTransactionReceipt;
    type Error = ScrollEthApiError;

    fn convert_receipts(
        &self,
        inputs: Vec<ConvertReceiptInput<'_, N>>,
    ) -> Result<Vec<Self::RpcReceipt>, Self::Error> {
        let mut receipts = Vec::with_capacity(inputs.len());

        for input in inputs {
            receipts.push(ScrollReceiptBuilder::new(input)?.build());
        }

        Ok(receipts)
    }
}

/// Builds an [`ScrollTransactionReceipt`].
#[derive(Debug)]
pub struct ScrollReceiptBuilder {
    /// Core receipt, has all the fields of an L1 receipt and is the basis for the Scroll receipt.
    pub core_receipt: TransactionReceipt<ScrollReceiptEnvelope<Log>>,
    /// Additional Scroll receipt fields.
    pub scroll_receipt_fields: ScrollTransactionReceiptFields,
}

impl ScrollReceiptBuilder {
    /// Returns a new builder.
    pub fn new<N>(input: ConvertReceiptInput<'_, N>) -> Result<Self, ScrollEthApiError>
    where
        N: NodePrimitives<SignedTx = ScrollTransactionSigned, Receipt = ScrollReceipt>,
    {
        let core_receipt =
            build_receipt(&input, None, |receipt_with_bloom| match input.receipt.as_ref() {
                ScrollReceipt::Legacy(_) => {
                    ScrollReceiptEnvelope::<Log>::Legacy(receipt_with_bloom)
                }
                ScrollReceipt::Eip2930(_) => {
                    ScrollReceiptEnvelope::<Log>::Eip2930(receipt_with_bloom)
                }
                ScrollReceipt::Eip1559(_) => {
                    ScrollReceiptEnvelope::<Log>::Eip1559(receipt_with_bloom)
                }
                ScrollReceipt::Eip7702(_) => {
                    ScrollReceiptEnvelope::<Log>::Eip7702(receipt_with_bloom)
                }
                ScrollReceipt::L1Message(_) => {
                    ScrollReceiptEnvelope::<Log>::L1Message(receipt_with_bloom)
                }
            });

        let scroll_receipt_fields =
            ScrollTransactionReceiptFields { l1_fee: Some(input.receipt.l1_fee().saturating_to()) };

        Ok(Self { core_receipt, scroll_receipt_fields })
    }

    /// Builds [`ScrollTransactionReceipt`] by combing core (l1) receipt fields and additional
    /// Scroll receipt fields.
    pub fn build(self) -> ScrollTransactionReceipt {
        let Self { core_receipt: inner, scroll_receipt_fields } = self;

        let ScrollTransactionReceiptFields { l1_fee, .. } = scroll_receipt_fields;

        ScrollTransactionReceipt { inner, l1_fee }
    }
}
