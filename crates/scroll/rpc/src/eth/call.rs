use crate::{ScrollEthApi, ScrollEthApiError};

use reth_evm::{SpecFor, TxEnvFor};
use reth_rpc_eth_api::{
    helpers::{estimate::EstimateCall, Call, EthCall},
    RpcConvert, RpcNodeCore,
};
use reth_rpc_eth_types::error::FromEvmError;

impl<N, Rpc> EthCall for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<
        Primitives = N::Primitives,
        Error = ScrollEthApiError,
        TxEnv = TxEnvFor<N::Evm>,
        Spec = SpecFor<N::Evm>,
    >,
{
}

impl<N, Rpc> EstimateCall for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<
        Primitives = N::Primitives,
        Error = ScrollEthApiError,
        TxEnv = TxEnvFor<N::Evm>,
        Spec = SpecFor<N::Evm>,
    >,
{
}

impl<N, Rpc> Call for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<
        Primitives = N::Primitives,
        Error = ScrollEthApiError,
        TxEnv = TxEnvFor<N::Evm>,
        Spec = SpecFor<N::Evm>,
    >,
{
    #[inline]
    fn call_gas_limit(&self) -> u64 {
        self.inner.eth_api.gas_cap()
    }

    #[inline]
    fn max_simulate_blocks(&self) -> u64 {
        self.inner.eth_api.max_simulate_blocks()
    }
}
