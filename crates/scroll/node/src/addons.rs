use crate::{
    builder::payload::SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT, ScrollEngineValidator,
    ScrollEngineValidatorBuilder, ScrollStorage,
};
use reth_evm::{ConfigureEvm, EvmFactory, EvmFactoryFor};
use reth_node_api::{AddOnsContext, NodeAddOns};
use reth_node_builder::{
    rpc::{
        BasicEngineApiBuilder, EngineValidatorAddOn, EngineValidatorBuilder, EthApiBuilder,
        Identity, RethRpcAddOns, RethRpcMiddleware, RpcAddOns, RpcHandle,
    },
    FullNodeComponents,
};
use reth_node_types::NodeTypes;
use reth_rpc_eth_types::error::FromEvmError;
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_engine_primitives::ScrollEngineTypes;
use reth_scroll_evm::ScrollNextBlockEnvAttributes;
use reth_scroll_primitives::ScrollPrimitives;
use reth_scroll_rpc::{eth::ScrollEthApiBuilder, ScrollEthApiError};
use revm::context::TxEnv;
use scroll_alloy_evm::ScrollTransactionIntoTxEnv;
use scroll_alloy_network::Scroll;
use std::marker::PhantomData;

/// Add-ons for the Scroll follower node.
#[derive(Debug)]
pub struct ScrollAddOns<N, RpcMiddleWare = Identity>
where
    N: FullNodeComponents,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    /// Rpc add-ons responsible for launching the RPC servers and instantiating the RPC handlers
    /// and eth-api.
    pub rpc_add_ons: RpcAddOns<
        N,
        ScrollEthApiBuilder,
        ScrollEngineValidatorBuilder,
        BasicEngineApiBuilder<ScrollEngineValidatorBuilder>,
        RpcMiddleWare,
    >,
}

impl<N> Default for ScrollAddOns<N, Identity>
where
    N: FullNodeComponents<Types: NodeTypes<Primitives = ScrollPrimitives>>,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    fn default() -> Self {
        Self::builder::<Scroll>().build()
    }
}

impl<N, RpcMiddleware> ScrollAddOns<N, RpcMiddleware>
where
    N: FullNodeComponents<Types: NodeTypes<Primitives = ScrollPrimitives>>,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    /// Build a [`ScrollAddOns`] using [`ScrollAddOnsBuilder`].
    pub fn builder<NetworkT>() -> ScrollAddOnsBuilder<NetworkT> {
        ScrollAddOnsBuilder::default()
    }
}

impl<N, RpcMiddleware> NodeAddOns<N> for ScrollAddOns<N, RpcMiddleware>
where
    N: FullNodeComponents<
        Types: NodeTypes<
            ChainSpec = ScrollChainSpec,
            Primitives = ScrollPrimitives,
            Storage = ScrollStorage,
            Payload = ScrollEngineTypes,
        >,
        Evm: ConfigureEvm<NextBlockEnvCtx = ScrollNextBlockEnvAttributes>,
    >,
    ScrollEthApiError: FromEvmError<N::Evm>,
    EvmFactoryFor<N::Evm>: EvmFactory<Tx = ScrollTransactionIntoTxEnv<TxEnv>>,
    RpcMiddleware: RethRpcMiddleware,
{
    type Handle = RpcHandle<N, <ScrollEthApiBuilder as EthApiBuilder<N>>::EthApi>;

    async fn launch_add_ons(self, ctx: AddOnsContext<'_, N>) -> eyre::Result<Self::Handle> {
        let Self { rpc_add_ons } = self;
        rpc_add_ons.launch_add_ons_with(ctx, |_| Ok(())).await
    }
}

impl<N, RpcMiddleware> RethRpcAddOns<N> for ScrollAddOns<N, RpcMiddleware>
where
    N: FullNodeComponents<
        Types: NodeTypes<
            ChainSpec = ScrollChainSpec,
            Primitives = ScrollPrimitives,
            Storage = ScrollStorage,
            Payload = ScrollEngineTypes,
        >,
        Evm: ConfigureEvm<NextBlockEnvCtx = ScrollNextBlockEnvAttributes>,
    >,
    ScrollEthApiError: FromEvmError<N::Evm>,
    EvmFactoryFor<N::Evm>: EvmFactory<Tx = ScrollTransactionIntoTxEnv<TxEnv>>,
    RpcMiddleware: RethRpcMiddleware,
{
    type EthApi = <ScrollEthApiBuilder as EthApiBuilder<N>>::EthApi;

    fn hooks_mut(&mut self) -> &mut reth_node_builder::rpc::RpcHooks<N, Self::EthApi> {
        self.rpc_add_ons.hooks_mut()
    }
}

impl<N, RpcMiddleware> EngineValidatorAddOn<N> for ScrollAddOns<N, RpcMiddleware>
where
    N: FullNodeComponents<
        Types: NodeTypes<
            ChainSpec = ScrollChainSpec,
            Primitives = ScrollPrimitives,
            Payload = ScrollEngineTypes,
        >,
    >,
    ScrollEthApiBuilder: EthApiBuilder<N>,
    RpcMiddleware: Send,
{
    type Validator = ScrollEngineValidator;

    async fn engine_validator(&self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator> {
        ScrollEngineValidatorBuilder.build(ctx).await
    }
}

/// A regular scroll evm and executor builder.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollAddOnsBuilder<NetworkT, RpcMiddleware = Identity> {
    /// Sequencer client, configured to forward submitted transactions to sequencer of given Scroll
    /// network.
    sequencer_url: Option<String>,
    /// Minimum suggested priority fee (tip)
    min_suggested_priority_fee: u64,
    /// Maximum payload size
    payload_size_limit: u64,
    /// Marker for network types.
    _nt: PhantomData<NetworkT>,
    /// RPC middleware to use
    rpc_middleware: RpcMiddleware,
}

impl<NetworkT> Default for ScrollAddOnsBuilder<NetworkT> {
    fn default() -> Self {
        Self {
            sequencer_url: None,
            payload_size_limit: SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT,
            // TODO (scroll): update with default values.
            min_suggested_priority_fee: 1_000_000,
            _nt: PhantomData,
            rpc_middleware: Identity::new(),
        }
    }
}

impl<NetworkT, RpcMiddleWare> ScrollAddOnsBuilder<NetworkT, RpcMiddleWare> {
    /// With a [`reth_scroll_rpc::SequencerClient`].
    pub fn with_sequencer(mut self, sequencer_client: Option<String>) -> Self {
        self.sequencer_url = sequencer_client;
        self
    }

    /// With minimum suggested priority fee.
    pub const fn with_min_suggested_priority_fee(
        mut self,
        min_suggested_priority_fee: u64,
    ) -> Self {
        self.min_suggested_priority_fee = min_suggested_priority_fee;
        self
    }

    /// With maximum payload size limit.
    pub const fn with_payload_size_limit(mut self, payload_size_limit: u64) -> Self {
        self.payload_size_limit = payload_size_limit;
        self
    }

    /// Configure the RPC middleware to use
    pub fn with_rpc_middleware<T>(self, rpc_middleware: T) -> ScrollAddOnsBuilder<NetworkT, T> {
        let Self { sequencer_url, min_suggested_priority_fee, payload_size_limit, _nt, .. } = self;
        ScrollAddOnsBuilder {
            sequencer_url,
            payload_size_limit,
            min_suggested_priority_fee,
            _nt,
            rpc_middleware,
        }
    }
}

impl<NetworkT, RpcMiddleWare> ScrollAddOnsBuilder<NetworkT, RpcMiddleWare> {
    /// Builds an instance of [`ScrollAddOns`].
    pub fn build<N>(self) -> ScrollAddOns<N, RpcMiddleWare>
    where
        N: FullNodeComponents<Types: NodeTypes<Primitives = ScrollPrimitives>>,
        ScrollEthApiBuilder: EthApiBuilder<N>,
    {
        let Self {
            sequencer_url,
            payload_size_limit,
            min_suggested_priority_fee,
            rpc_middleware,
            ..
        } = self;

        ScrollAddOns {
            rpc_add_ons: RpcAddOns::new(
                ScrollEthApiBuilder::new()
                    .with_sequencer(sequencer_url)
                    .with_payload_size_limit(payload_size_limit)
                    .with_min_suggested_priority_fee(min_suggested_priority_fee),
                Default::default(),
                Default::default(),
                rpc_middleware,
            ),
        }
    }
}
