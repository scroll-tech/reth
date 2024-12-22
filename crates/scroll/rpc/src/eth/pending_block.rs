//! Loads Scroll pending block for an RPC response.

use alloy_consensus::Header;
use reth_chainspec::{EthChainSpec, EthereumHardforks};
use reth_evm::ConfigureEvm;
use reth_primitives::Receipt;
use reth_provider::{BlockReaderIdExt, ChainSpecProvider, EvmEnvProvider, StateProviderFactory};
use reth_rpc_eth_api::{
    helpers::{LoadPendingBlock, SpawnBlocking},
    RpcNodeCore,
};
use reth_rpc_eth_types::PendingBlock;
use reth_transaction_pool::TransactionPool;

use crate::ScrollEthApi;

impl<N> LoadPendingBlock for ScrollEthApi<N>
where
    Self: SpawnBlocking,
    N: RpcNodeCore<
        Provider: BlockReaderIdExt<
            Block = reth_primitives::Block,
            Receipt = reth_primitives::Receipt,
        > + EvmEnvProvider
                      + ChainSpecProvider<ChainSpec: EthChainSpec + EthereumHardforks>
                      + StateProviderFactory,
        Pool: TransactionPool,
        Evm: ConfigureEvm<Header = Header>,
    >,
{
    #[inline]
    fn pending_block(&self) -> &tokio::sync::Mutex<Option<PendingBlock>> {
        self.inner.pending_block()
    }
}
