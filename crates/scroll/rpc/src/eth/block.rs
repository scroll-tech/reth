//! Loads and formats Scroll block RPC response.

use alloy_rpc_types_eth::BlockId;
use reth_chainspec::ChainSpecProvider;
use reth_primitives::TransactionMeta;
use reth_provider::HeaderProvider;
use reth_rpc_eth_api::{
    helpers::{EthBlocks, LoadBlock, LoadPendingBlock, LoadReceipt, SpawnBlocking},
    RpcNodeCore, RpcReceipt,
};
use reth_rpc_eth_types::EthReceiptBuilder;
use reth_scroll_chainspec::ScrollChainSpec;

use scroll_alloy_network::Network;
use scroll_alloy_rpc_types::ScrollTransactionReceipt;

use crate::{ScrollEthApi, ScrollEthApiError};

impl<N> EthBlocks for ScrollEthApi<N>
where
    Self: LoadBlock<
        Error = ScrollEthApiError,
        NetworkTypes: Network<ReceiptResponse = ScrollTransactionReceipt>,
    >,
    N: RpcNodeCore<Provider: ChainSpecProvider<ChainSpec = ScrollChainSpec> + HeaderProvider>,
{
    async fn block_receipts(
        &self,
        block_id: BlockId,
    ) -> Result<Option<Vec<RpcReceipt<Self::NetworkTypes>>>, Self::Error>
    where
        Self: LoadReceipt,
    {
        if let Some((block, receipts)) = self.load_block_and_receipts(block_id).await? {
            let block_number = block.number;
            let base_fee = block.base_fee_per_gas;
            let block_hash = block.hash();
            let excess_blob_gas = block.excess_blob_gas;
            let timestamp = block.timestamp;

            return block
                .body
                .transactions
                .into_iter()
                .zip(receipts.iter())
                .enumerate()
                .map(|(idx, (ref tx, receipt))| -> Result<_, _> {
                    let meta = TransactionMeta {
                        tx_hash: tx.hash(),
                        index: idx as u64,
                        block_hash,
                        block_number,
                        base_fee,
                        excess_blob_gas,
                        timestamp,
                    };

                    EthReceiptBuilder::new(&tx, meta, receipt, &receipts)
                        .map(|builder| builder.build())
                })
                .collect::<Result<Vec<_>, Self::Error>>()
                .map(Some)
        }

        Ok(None)
    }
}

impl<N> LoadBlock for ScrollEthApi<N>
where
    Self: LoadPendingBlock + SpawnBlocking,
    N: RpcNodeCore,
{
}
