//! Loads and formats Scroll transaction RPC response.

use alloy_consensus::{Signed, Transaction as _};
use alloy_primitives::{Bytes, Sealable, Sealed, B256};
use alloy_rpc_types_eth::{Transaction, TransactionInfo};
use reth_node_api::FullNodeComponents;
use reth_primitives::{TransactionSigned, TransactionSignedEcRecovered};
use reth_provider::{BlockReaderIdExt, ReceiptProvider, TransactionsProvider};
use reth_rpc_eth_api::{
    helpers::{EthSigner, EthTransactions, LoadTransaction, SpawnBlocking},
    FromEthApiError, FullEthApiTypes, RpcNodeCore, TransactionCompat,
};
use reth_rpc_eth_types::utils::recover_raw_transaction;
use reth_transaction_pool::{PoolTransaction, TransactionOrigin, TransactionPool};

use scroll_alloy_consensus::ScrollTxEnvelope;

use crate::{ScrollEthApi, ScrollEthApiError};

impl<N> EthTransactions for ScrollEthApi<N>
where
    Self: LoadTransaction<Provider: BlockReaderIdExt>,
    N: RpcNodeCore,
{
    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn EthSigner>>> {
        self.inner.signers()
    }
}

impl<N> LoadTransaction for ScrollEthApi<N>
where
    Self: SpawnBlocking + FullEthApiTypes,
    N: RpcNodeCore<Provider: TransactionsProvider, Pool: TransactionPool>,
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
        tx: TransactionSignedEcRecovered,
        tx_info: TransactionInfo,
    ) -> Result<Self::Transaction, Self::Error> {
        let from = tx.signer();
        let hash = tx.hash();
        let TransactionSigned { transaction, signature, .. } = tx.into_signed();

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
            reth_primitives::Transaction::Eip4844(_) => unreachable!(),
            reth_primitives::Transaction::Eip7702(tx) => unreachable!(),
            reth_primitives::Transaction::Deposit(tx) => {
                ScrollTxEnvelope::L1Message(tx.seal_unchecked(hash))
            }
        };

        let TransactionInfo {
            block_hash, block_number, index: transaction_index, base_fee, ..
        } = tx_info;

        let effective_gas_price = if inner.is_deposit() {
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
            inner,
            block_hash,
            block_number,
            transaction_index,
            from,
            effective_gas_price: Some(effective_gas_price),
        })
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
