//! Loads and formats Scroll block RPC response.

use crate::{ScrollEthApi, ScrollEthApiError};

use reth_rpc_convert::RpcConvert;
use reth_rpc_eth_api::{
    helpers::{EthBlocks, LoadBlock},
    RpcNodeCore,
};
use reth_rpc_eth_types::error::FromEvmError;

impl<N, Rpc> EthBlocks for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}

impl<N, Rpc> LoadBlock for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}
