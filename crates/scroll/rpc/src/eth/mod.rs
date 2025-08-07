//! Scroll-Reth `eth_` endpoint implementation.

use crate::{
    eth::{receipt::ScrollReceiptConverter, transaction::ScrollTxInfoMapper},
    ScrollEthApiError, SequencerClient,
};
use alloy_primitives::U256;
use eyre::WrapErr;
pub use receipt::ScrollReceiptBuilder;
use reth_evm::ConfigureEvm;
use reth_node_api::{FullNodeComponents, FullNodeTypes, HeaderTy};
use reth_node_builder::rpc::{EthApiBuilder, EthApiCtx};
use reth_provider::{BlockReader, ProviderHeader, ProviderTx};
use reth_rpc::eth::{core::EthApiInner, DevSigner};
use reth_rpc_convert::{RpcConvert, RpcConverter, RpcTypes, SignableTxRequest};
use reth_rpc_eth_api::{
    helpers::{
        pending_block::BuildPendingEnv, spec::SignersForApi, AddDevSigners, EthApiSpec, EthState,
        LoadFee, LoadState, SpawnBlocking, Trace,
    },
    EthApiTypes, FullEthApiServer, RpcNodeCore, RpcNodeCoreExt,
};
use reth_rpc_eth_types::{error::FromEvmError, EthStateCache, FeeHistoryCache, GasPriceOracle};
use reth_tasks::{
    pool::{BlockingTaskGuard, BlockingTaskPool},
    TaskSpawner,
};
use scroll_alloy_network::Scroll;
use std::{fmt, marker::PhantomData, sync::Arc};

mod block;
mod call;
mod fee;
mod pending_block;
pub mod receipt;
pub mod transaction;

/// Adapter for [`EthApiInner`], which holds all the data required to serve core `eth_` API.
pub type EthApiNodeBackend<N, Rpc> = EthApiInner<N, Rpc>;

/// A helper trait with requirements for [`RpcNodeCore`] to be used in [`ScrollEthApi`].
pub trait ScrollNodeCore: RpcNodeCore<Provider: BlockReader> {}
impl<T> ScrollNodeCore for T where T: RpcNodeCore<Provider: BlockReader> {}

/// Scroll-Reth `Eth` API implementation.
///
/// This type provides the functionality for handling `eth_` related requests.
///
/// This wraps a default `Eth` implementation, and provides additional functionality where the
/// scroll spec deviates from the default (ethereum) spec, e.g. transaction forwarding to the
/// receipts, additional RPC fields for transaction receipts.
///
/// This type implements the [`FullEthApi`](reth_rpc_eth_api::helpers::FullEthApi) by implemented
/// all the `Eth` helper traits and prerequisite traits.
#[derive(Clone)]
pub struct ScrollEthApi<N: RpcNodeCore, Rpc: RpcConvert> {
    /// Gateway to node's core components.
    inner: Arc<ScrollEthApiInner<N, Rpc>>,
}

impl<N: RpcNodeCore, Rpc: RpcConvert> ScrollEthApi<N, Rpc> {
    /// Creates a new [`ScrollEthApi`].
    pub fn new(
        eth_api: EthApiNodeBackend<N, Rpc>,
        sequencer_client: Option<SequencerClient>,
        min_suggested_priority_fee: U256,
        payload_size_limit: u64,
        propagate_local_transactions: bool,
    ) -> Self {
        let inner = Arc::new(ScrollEthApiInner {
            eth_api,
            min_suggested_priority_fee,
            payload_size_limit,
            sequencer_client,
            propagate_local_transactions,
        });
        Self { inner }
    }
}

impl<N, Rpc> ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    /// Returns a reference to the [`EthApiNodeBackend`].
    pub fn eth_api(&self) -> &EthApiNodeBackend<N, Rpc> {
        self.inner.eth_api()
    }

    /// Returns the configured sequencer client, if any.
    pub fn sequencer_client(&self) -> Option<&SequencerClient> {
        self.inner.sequencer_client()
    }

    /// Return a builder for the [`ScrollEthApi`].
    pub fn builder() -> ScrollEthApiBuilder {
        ScrollEthApiBuilder::new()
    }
}

impl<N, Rpc> EthApiTypes for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    type Error = ScrollEthApiError;
    type NetworkTypes = Rpc::Network;
    type RpcConvert = Rpc;

    fn tx_resp_builder(&self) -> &Self::RpcConvert {
        self.inner.eth_api.tx_resp_builder()
    }
}

impl<N, Rpc> RpcNodeCore for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    type Primitives = N::Primitives;
    type Provider = N::Provider;
    type Pool = N::Pool;
    type Evm = <N as RpcNodeCore>::Evm;
    type Network = <N as RpcNodeCore>::Network;

    #[inline]
    fn pool(&self) -> &Self::Pool {
        self.inner.eth_api.pool()
    }

    #[inline]
    fn evm_config(&self) -> &Self::Evm {
        self.inner.eth_api.evm_config()
    }

    #[inline]
    fn network(&self) -> &Self::Network {
        self.inner.eth_api.network()
    }

    #[inline]
    fn provider(&self) -> &Self::Provider {
        self.inner.eth_api.provider()
    }
}

impl<N, Rpc> RpcNodeCoreExt for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    #[inline]
    fn cache(&self) -> &EthStateCache<N::Primitives> {
        self.inner.eth_api.cache()
    }
}

impl<N, Rpc> EthApiSpec for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    type Transaction = ProviderTx<Self::Provider>;
    type Rpc = Rpc::Network;

    #[inline]
    fn starting_block(&self) -> U256 {
        self.inner.eth_api.starting_block()
    }

    #[inline]
    fn signers(&self) -> &SignersForApi<Self> {
        self.inner.eth_api.signers()
    }
}

impl<N, Rpc> SpawnBlocking for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    #[inline]
    fn io_task_spawner(&self) -> impl TaskSpawner {
        self.inner.eth_api.task_spawner()
    }

    #[inline]
    fn tracing_task_pool(&self) -> &BlockingTaskPool {
        self.inner.eth_api.blocking_task_pool()
    }

    #[inline]
    fn tracing_task_guard(&self) -> &BlockingTaskGuard {
        self.inner.eth_api.blocking_task_guard()
    }
}

impl<N, Rpc> LoadFee for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
    #[inline]
    fn gas_oracle(&self) -> &GasPriceOracle<Self::Provider> {
        self.inner.eth_api.gas_oracle()
    }

    #[inline]
    fn fee_history_cache(&self) -> &FeeHistoryCache<ProviderHeader<N::Provider>> {
        self.inner.eth_api.fee_history_cache()
    }

    async fn suggested_priority_fee(&self) -> Result<U256, Self::Error> {
        let min_tip = U256::from(self.inner.min_suggested_priority_fee);
        self.inner
            .eth_api
            .gas_oracle()
            .scroll_suggest_tip_cap(min_tip, self.inner.payload_size_limit)
            .await
            .map_err(Into::into)
    }
}

impl<N, Rpc> LoadState for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
}

impl<N, Rpc> EthState for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<Primitives = N::Primitives>,
{
    #[inline]
    fn max_proof_window(&self) -> u64 {
        self.inner.eth_api.eth_proof_window()
    }
}

impl<N, Rpc> Trace for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    ScrollEthApiError: FromEvmError<N::Evm>,
    Rpc: RpcConvert<Primitives = N::Primitives, Error = ScrollEthApiError>,
{
}

impl<N, Rpc> AddDevSigners for ScrollEthApi<N, Rpc>
where
    N: RpcNodeCore,
    Rpc: RpcConvert<
        Network: RpcTypes<TransactionRequest: SignableTxRequest<ProviderTx<N::Provider>>>,
    >,
{
    fn with_dev_accounts(&self) {
        *self.inner.eth_api.signers().write() = DevSigner::random_signers(20)
    }
}

impl<N: ScrollNodeCore, Rpc: RpcConvert> fmt::Debug for ScrollEthApi<N, Rpc> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrollEthApi").finish_non_exhaustive()
    }
}

/// Container type `ScrollEthApi`
#[allow(missing_debug_implementations)]
pub struct ScrollEthApiInner<N: ScrollNodeCore, Rpc: RpcConvert> {
    /// Gateway to node's core components.
    pub eth_api: EthApiNodeBackend<N, Rpc>,
    /// Sequencer client, configured to forward submitted transactions to sequencer of given Scroll
    /// network.
    sequencer_client: Option<SequencerClient>,
    /// Minimum priority fee
    min_suggested_priority_fee: U256,
    /// Maximum payload size
    payload_size_limit: u64,
    /// whether local transactions should be propagated.
    propagate_local_transactions: bool,
}

impl<N: RpcNodeCore, Rpc: RpcConvert> ScrollEthApiInner<N, Rpc> {
    /// Returns a reference to the [`EthApiNodeBackend`].
    const fn eth_api(&self) -> &EthApiNodeBackend<N, Rpc> {
        &self.eth_api
    }

    /// Returns the configured sequencer client, if any.
    const fn sequencer_client(&self) -> Option<&SequencerClient> {
        self.sequencer_client.as_ref()
    }
}

/// Converter for Scroll RPC types.
pub type ScrollRpcConvert<N, NetworkT> = RpcConverter<
    NetworkT,
    <N as FullNodeComponents>::Evm,
    ScrollReceiptConverter,
    (),
    ScrollTxInfoMapper<<N as FullNodeTypes>::Provider>,
>;

/// The default suggested priority fee for the gas price oracle.
pub const DEFAULT_MIN_SUGGESTED_PRIORITY_FEE: u64 = 100;

/// The default payload size limit in bytes for the sequencer.
pub const DEFAULT_PAYLOAD_SIZE_LIMIT: u64 = 122_880;

/// A type that knows how to build a [`ScrollEthApi`].
#[derive(Debug)]
pub struct ScrollEthApiBuilder<NetworkT = Scroll> {
    /// Sequencer client, configured to forward submitted transactions to sequencer of given Scroll
    /// network.
    sequencer_url: Option<String>,
    /// Minimum suggested priority fee (tip)
    min_suggested_priority_fee: u64,
    /// Maximum payload size
    payload_size_limit: u64,
    /// whether local transactions should be propagated.
    propagate_local_transactions: bool,
    /// Marker for network types.
    _nt: PhantomData<NetworkT>,
}

impl<NetworkT> Default for ScrollEthApiBuilder<NetworkT> {
    fn default() -> Self {
        Self {
            sequencer_url: None,
            min_suggested_priority_fee: DEFAULT_MIN_SUGGESTED_PRIORITY_FEE,
            payload_size_limit: DEFAULT_PAYLOAD_SIZE_LIMIT,
            propagate_local_transactions: true,
            _nt: PhantomData,
        }
    }
}

impl<NetworkT> ScrollEthApiBuilder<NetworkT> {
    /// Creates a [`ScrollEthApiBuilder`] instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// With a [`SequencerClient`].
    pub fn with_sequencer(mut self, sequencer_url: Option<String>) -> Self {
        self.sequencer_url = sequencer_url;
        self
    }

    /// With minimum suggested priority fee (tip)
    pub const fn with_min_suggested_priority_fee(mut self, min: u64) -> Self {
        self.min_suggested_priority_fee = min;
        self
    }

    /// With payload size limit
    pub const fn with_payload_size_limit(mut self, limit: u64) -> Self {
        self.payload_size_limit = limit;
        self
    }

    /// With whether local transactions should be propagated.
    pub const fn with_propagate_local_transactions(
        &mut self,
        propagate_local_transactions: bool,
    ) -> &mut Self {
        self.propagate_local_transactions = propagate_local_transactions;
        self
    }
}

impl<N, NetworkT> EthApiBuilder<N> for ScrollEthApiBuilder<NetworkT>
where
    N: FullNodeComponents<Evm: ConfigureEvm<NextBlockEnvCtx: BuildPendingEnv<HeaderTy<N::Types>>>>,
    NetworkT: RpcTypes,
    ScrollRpcConvert<N, NetworkT>: RpcConvert<Network = NetworkT>,
    ScrollEthApi<N, ScrollRpcConvert<N, NetworkT>>:
        FullEthApiServer<Provider = N::Provider, Pool = N::Pool> + AddDevSigners,
{
    type EthApi = ScrollEthApi<N, ScrollRpcConvert<N, NetworkT>>;

    async fn build_eth_api(self, ctx: EthApiCtx<'_, N>) -> eyre::Result<Self::EthApi> {
        let Self {
            min_suggested_priority_fee,
            payload_size_limit,
            sequencer_url,
            propagate_local_transactions,
            ..
        } = self;
        let rpc_converter = RpcConverter::new(ScrollReceiptConverter::default())
            .with_mapper(ScrollTxInfoMapper::new(ctx.components.provider().clone()));

        let eth_api = ctx.eth_api_builder().with_rpc_converter(rpc_converter).build_inner();

        let sequencer_client = if let Some(url) = sequencer_url {
            Some(
                SequencerClient::new(&url)
                    .await
                    .wrap_err_with(|| "Failed to init sequencer client with: {url}")?,
            )
        } else {
            None
        };

        Ok(ScrollEthApi::new(
            eth_api,
            sequencer_client,
            U256::from(min_suggested_priority_fee),
            payload_size_limit,
            propagate_local_transactions,
        ))
    }
}
