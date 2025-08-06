//! Loads Scroll pending block for an RPC response.

use crate::{ScrollEthApi, ScrollEthApiError};
use reth_rpc_eth_api::{
    helpers::{pending_block::PendingEnvBuilder, LoadPendingBlock},
    RpcConvert, RpcNodeCore,
};
use reth_rpc_eth_types::{error::FromEvmError, PendingBlock};

impl<N, Rpc> LoadPendingBlock for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    #[inline]
    fn pending_block(&self) -> &tokio::sync::Mutex<Option<PendingBlock<N::Primitives>>> {
        self.inner.eth_api.pending_block()
    }

    #[inline]
    fn pending_env_builder(&self) -> &dyn PendingEnvBuilder<Self::Evm> {
        self.inner.eth_api.pending_env_builder()
    }
}
