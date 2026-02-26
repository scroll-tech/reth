//! Loads and formats Scroll transaction RPC response.

use crate::{ScrollEthApi, ScrollEthApiError, SequencerClient};
use alloy_consensus::transaction::TransactionInfo;
use alloy_primitives::B256;
use reth_evm::execute::ProviderError;
use reth_primitives_traits::{Recovered, WithEncoded};
use reth_provider::ReceiptProvider;
use reth_rpc_convert::RpcConvert;
use reth_rpc_eth_api::{
    helpers::{spec::SignersForRpc, EthTransactions, LoadTransaction},
    try_into_scroll_tx_info, FromEthApiError, RpcNodeCore, TxInfoMapper,
};
use reth_scroll_primitives::ScrollReceipt;
use reth_transaction_pool::{
    AddedTransactionOutcome, PoolPooledTx, PoolTransaction, TransactionOrigin, TransactionPool,
};
use scroll_alloy_consensus::{ScrollTransactionInfo, ScrollTxEnvelope};
use std::{
    fmt::{Debug, Formatter},
    time::Duration,
};

impl<N, Rpc> EthTransactions for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
    fn signers(&self) -> &SignersForRpc<Self::Provider, Self::NetworkTypes> {
        self.inner.eth_api.signers()
    }

    fn send_raw_transaction_sync_timeout(&self) -> Duration {
        self.inner.eth_api.send_raw_transaction_sync_timeout()
    }

    async fn send_transaction(
        &self,
        origin: TransactionOrigin,
        tx: WithEncoded<Recovered<PoolPooledTx<Self::Pool>>>,
    ) -> Result<B256, Self::Error> {
        let (raw_tx, recovered) = tx.split();
        let pool_transaction = <Self::Pool as TransactionPool>::Transaction::from_pooled(recovered);

        // submit the transaction to the pool
        let AddedTransactionOutcome { hash, .. } = self
            .pool()
            .add_transaction(origin, pool_transaction.clone())
            .await
            .map_err(Self::Error::from_eth_err)?;

        // On scroll, transactions are forwarded directly to the sequencer to be included in
        // blocks that it builds.
        if let Some(client) = self.raw_tx_forwarder() {
            tracing::debug!(target: "scroll::rpc::eth", hash = %pool_transaction.hash(), "forwarding raw transaction to sequencer");

            if self.inner.propagate_local_transactions {
                // Forward to remote sequencer RPC asynchronously (fire and forget)
                let client = client.clone();
                tokio::spawn(async move {
                    match client.forward_raw_transaction(&raw_tx).await {
                        Ok(sequencer_hash) => {
                            tracing::debug!(target: "scroll::rpc::eth", local_hash=%hash, %sequencer_hash, "successfully forwarded transaction to sequencer");
                        }
                        Err(err) => {
                            tracing::warn!(target: "scroll::rpc::eth", %err, local_hash=%hash, "failed to forward transaction to sequencer, but transaction is in local pool and will be propagated");
                        }
                    }
                });
            } else {
                // Forward to remote sequencer RPC synchronously
                match client.forward_raw_transaction(&raw_tx).await {
                    Ok(sequencer_hash) => {
                        tracing::debug!(target: "scroll::rpc::eth", local_hash=%hash, %sequencer_hash, "successfully forwarded transaction to sequencer");
                    }
                    Err(err) => {
                        tracing::warn!(target: "scroll::rpc::eth", %err, local_hash=%hash, "failed to forward transaction to sequencer");
                        return Err(ScrollEthApiError::Sequencer(err));
                    }
                }
            }
        }

        Ok(hash)
    }
}

impl<N, Rpc> LoadTransaction for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}

impl<N, Rpc> ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    /// Returns the [`SequencerClient`] if one is set.
    pub fn raw_tx_forwarder(&self) -> Option<SequencerClient> {
        self.inner.sequencer_client.clone()
    }
}

/// Scroll implementation of [`TxInfoMapper`].
///
/// Receipt is fetched to extract the `l1_fee` for all transactions but L1 messages.
pub struct ScrollTxInfoMapper<Provider>(Provider);

impl<Provider: Clone> Clone for ScrollTxInfoMapper<Provider> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Provider: Debug> Debug for ScrollTxInfoMapper<Provider> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollTxInfoMapper").finish()
    }
}

impl<Provider> ScrollTxInfoMapper<Provider> {
    /// Creates [`ScrollTxInfoMapper`] that uses [`ReceiptProvider`] borrowed from given `eth_api`.
    pub const fn new(provider: Provider) -> Self {
        Self(provider)
    }
}

impl<Provider> TxInfoMapper<ScrollTxEnvelope> for ScrollTxInfoMapper<Provider>
where
    Provider: ReceiptProvider<Receipt = ScrollReceipt>,
{
    type Out = ScrollTransactionInfo;
    type Err = ProviderError;

    fn try_map(
        &self,
        tx: &ScrollTxEnvelope,
        tx_info: TransactionInfo,
    ) -> Result<Self::Out, ProviderError> {
        try_into_scroll_tx_info(&self.0, tx, tx_info)
    }
}
