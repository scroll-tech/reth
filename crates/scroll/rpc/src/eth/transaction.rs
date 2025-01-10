//! Loads and formats Scroll transaction RPC response.

use alloy_consensus::{Signed, Transaction as _};
use alloy_primitives::{PrimitiveSignature as Signature, Sealable, Sealed};
use alloy_rpc_types_eth::TransactionInfo;
use reth_node_api::FullNodeComponents;
use reth_primitives::{RecoveredTx, TransactionSigned};
use reth_provider::{ReceiptProvider, TransactionsProvider};
use reth_rpc_eth_api::{
    helpers::{LoadTransaction, SpawnBlocking},
    FullEthApiTypes, RpcNodeCoreExt, TransactionCompat,
};
use reth_rpc_eth_types::EthApiError;
use reth_transaction_pool::TransactionPool;

use scroll_alloy_consensus::ScrollTxEnvelope;
use scroll_alloy_rpc_types::Transaction;

use crate::{eth::ScrollNodeCore, ScrollEthApi, ScrollEthApiError};

impl<N> LoadTransaction for ScrollEthApi<N>
where
    Self: SpawnBlocking + FullEthApiTypes + RpcNodeCoreExt,
    N: ScrollNodeCore<Provider: TransactionsProvider, Pool: TransactionPool>,
    Self::Pool: TransactionPool,
{
}

impl<N> TransactionCompat for ScrollEthApi<N>
where
    N: FullNodeComponents<Provider: ReceiptProvider<Receipt = reth_primitives::Receipt>>,
{
    type Transaction = Transaction;
    type Error = ScrollEthApiError;

    fn fill(
        &self,
        tx: RecoveredTx,
        tx_info: TransactionInfo,
    ) -> Result<Self::Transaction, Self::Error> {
        let from = tx.signer();
        let hash = tx.hash();
        let TransactionSigned { transaction, signature, .. } = tx.into_signed();
        let mut tx_sender = None;
        let mut tx_queue_index = None;

        let inner = match transaction {
            reth_primitives::Transaction::Legacy(tx) => {
                Signed::new_unchecked(tx, signature, hash).into()
            }
            reth_primitives::Transaction::Eip2930(tx) => {
                Signed::new_unchecked(tx, signature, hash).into()
            }
            reth_primitives::Transaction::Eip1559(tx) => {
                Signed::new_unchecked(tx, signature, hash).into()
            }
            reth_primitives::Transaction::Eip4844(_) | reth_primitives::Transaction::Eip7702(_) => {
                unreachable!()
            }
            reth_primitives::Transaction::L1Message(tx) => {
                tx_queue_index = Some(tx.queue_index);
                tx_sender = Some(tx.sender);

                ScrollTxEnvelope::L1Message(tx.seal_unchecked(hash))
            }
        };

        let TransactionInfo {
            block_hash, block_number, index: transaction_index, base_fee, ..
        } = tx_info;

        let effective_gas_price = if inner.is_l1_message() {
            // For deposits, we must always set the `gasPrice` field to 0 in rpc
            // deposit tx don't have a gas price field, but serde of `Transaction` will take care of
            // it
            0
        } else {
            base_fee
                .map(|base_fee| {
                    inner.effective_tip_per_gas(base_fee as u64).unwrap_or_default() + base_fee
                })
                .unwrap_or_else(|| inner.max_fee_per_gas())
        };

        Ok(Transaction {
            inner: alloy_rpc_types_eth::Transaction {
                inner,
                block_hash,
                block_number,
                transaction_index,
                from,
                effective_gas_price: Some(effective_gas_price),
            },
            sender: tx_sender,
            queue_index: tx_queue_index,
        })
    }

    fn build_simulate_v1_transaction(
        &self,
        request: alloy_rpc_types_eth::TransactionRequest,
    ) -> Result<TransactionSigned, Self::Error> {
        let Ok(tx) = request.build_typed_tx() else {
            return Err(ScrollEthApiError::Eth(EthApiError::TransactionConversionError))
        };

        // Create an empty signature for the transaction.
        let signature = Signature::new(Default::default(), Default::default(), false);
        Ok(TransactionSigned::new_unhashed(tx.into(), signature))
    }

    fn otterscan_api_truncate_input(tx: &mut Self::Transaction) {
        let input = match &mut tx.inner.inner {
            ScrollTxEnvelope::Eip1559(tx) => &mut tx.tx_mut().input,
            ScrollTxEnvelope::Eip2930(tx) => &mut tx.tx_mut().input,
            ScrollTxEnvelope::Legacy(tx) => &mut tx.tx_mut().input,
            ScrollTxEnvelope::L1Message(tx) => {
                let (mut deposit, hash) = std::mem::replace(
                    tx,
                    Sealed::new_unchecked(Default::default(), Default::default()),
                )
                .split();
                deposit.input = deposit.input.slice(..4);
                let mut deposit = deposit.seal_unchecked(hash);
                std::mem::swap(tx, &mut deposit);
                return
            }
            _ => return,
        };
        *input = input.slice(..4);
    }
}
