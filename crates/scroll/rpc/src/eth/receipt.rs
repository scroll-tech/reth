//! Loads and formats Scroll receipt RPC response.

use alloy_eips::eip2718::Encodable2718;
use alloy_rpc_types_eth::{Log, TransactionReceipt};
use reth_node_api::{FullNodeComponents, NodeTypes};
use reth_primitives::{Receipt, TransactionMeta, TransactionSigned, TxType};
use reth_provider::{ChainSpecProvider, ReceiptProvider, TransactionsProvider};
use reth_rpc_eth_api::{helpers::LoadReceipt, FromEthApiError, RpcReceipt};
use reth_rpc_eth_types::{receipt::build_receipt, EthApiError};

use reth_scroll_chainspec::ScrollChainSpec;

use crate::{ScrollEthApi, ScrollEthApiError};

impl<N> LoadReceipt for ScrollEthApi<N>
where
    Self: Send + Sync,
    N: FullNodeComponents<Types: NodeTypes<ChainSpec = ScrollChainSpec>>,
    Self::Provider:
        TransactionsProvider<Transaction = TransactionSigned> + ReceiptProvider<Receipt = Receipt>,
{
    async fn build_transaction_receipt(
        &self,
        tx: TransactionSigned,
        meta: TransactionMeta,
        receipt: Receipt,
    ) -> Result<RpcReceipt<Self::NetworkTypes>, Self::Error> {
        let (block, receipts) = self
            .cache()
            .get_block_and_receipts(meta.block_hash)
            .await
            .map_err(Self::Error::from_eth_err)?
            .ok_or(Self::Error::from_eth_err(EthApiError::HeaderNotFound(
                meta.block_hash.into(),
            )))?;

        // todo
        let l1_block_info =
            reth_optimism_evm::extract_l1_info(&block.body).map_err(OpEthApiError::from)?;

        Ok(OpReceiptBuilder::new(
            &self.inner.provider().chain_spec(),
            &tx,
            meta,
            &receipt,
            &receipts,
            l1_block_info,
        )?
        .build())
    }
}
