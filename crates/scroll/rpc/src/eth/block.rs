//! Loads and formats Scroll block RPC response.

use crate::{RpcBlockHeaderMut, ScrollEthApi, ScrollEthApiError};

use alloy_eips::BlockId;
use reth_rpc_convert::{RpcConvert, RpcTypes};
use reth_rpc_eth_api::{
    helpers::{EthBlocks, LoadBlock},
    EthApiTypes, FullEthApiTypes, RpcBlock, RpcNodeCore,
};
use reth_rpc_eth_types::error::FromEvmError;

impl<N, Rpc> EthBlocks for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
    <<Self as EthApiTypes>::NetworkTypes as RpcTypes>::Header: RpcBlockHeaderMut,
{
    async fn rpc_block(
        &self,
        block_id: BlockId,
        full: bool,
    ) -> Result<Option<RpcBlock<Self::NetworkTypes>>, Self::Error>
    where
        Self: FullEthApiTypes,
    {
        let Some(block) = self.recovered_block(block_id).await? else { return Ok(None) };

        let mut block = block.clone_into_rpc_block(
            full.into(),
            |tx, tx_info| self.converter().fill(tx, tx_info),
            |header, size| self.converter().convert_header(header, size),
        )?;

        *block.header.total_difficulty_mut() = None;

        Ok(Some(block))
    }
}

impl<N, Rpc> LoadBlock for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}
