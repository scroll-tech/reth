use crate::{
    builder::{engine::ScrollEngineValidatorBuilder, payload::SCROLL_DEFAULT_PAYLOAD_SIZE_LIMIT},
    ScrollStorage,
};
use reth_evm::{ConfigureEngineEvm, EvmFactory, EvmFactoryFor};
use reth_node_api::{AddOnsContext, NodeAddOns, PayloadTypes};
use reth_node_builder::{
    rpc::{
        BasicEngineApiBuilder, BasicEngineValidatorBuilder, EngineValidatorAddOn, EthApiBuilder,
        Identity, RethRpcAddOns, RethRpcMiddleware, RpcAddOns, RpcHandle,
    },
    FullNodeComponents,
};
use reth_node_types::NodeTypes;
use reth_revm::context::BlockEnv;
use reth_rpc_eth_types::error::FromEvmError;
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_engine_primitives::ScrollEngineTypes;
use reth_scroll_evm::ScrollNextBlockEnvAttributes;
use reth_scroll_primitives::ScrollPrimitives;
use reth_scroll_rpc::{
    eth::{ScrollEthApiBuilder, DEFAULT_MIN_SUGGESTED_PRIORITY_FEE},
    ScrollEthApiError,
};
use revm::context::TxEnv;
use scroll_alloy_evm::ScrollTransactionIntoTxEnv;
use scroll_alloy_hardforks::ScrollHardforks;
use scroll_alloy_network::Scroll;
use std::marker::PhantomData;

/// Marker trait for Scroll node types with standard engine, chain spec, and primitives.
pub trait ScrollNodeTypes:
    NodeTypes<Payload = ScrollEngineTypes, ChainSpec: ScrollHardforks, Primitives = ScrollPrimitives>
{
}

/// Blanket impl for all node types that conform to the Scroll spec.
impl<N> ScrollNodeTypes for N where
    N: NodeTypes<
        Payload = ScrollEngineTypes,
        ChainSpec: ScrollHardforks,
        Primitives = ScrollPrimitives,
    >
{
}

/// Add-ons for the Scroll follower node.
#[derive(Debug)]
pub struct ScrollAddOns<N, RpcMiddleWare = Identity>
where
    N: FullNodeComponents<Types: ScrollNodeTypes>,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    /// Rpc add-ons responsible for launching the RPC servers and instantiating the RPC handlers
    /// and eth-api.
    pub rpc_add_ons: RpcAddOns<
        N,
        ScrollEthApiBuilder,
        ScrollEngineValidatorBuilder,
        BasicEngineApiBuilder<ScrollEngineValidatorBuilder>,
        BasicEngineValidatorBuilder<ScrollEngineValidatorBuilder>,
        RpcMiddleWare,
    >,
}

impl<N> Default for ScrollAddOns<N, Identity>
where
    N: FullNodeComponents<Types: ScrollNodeTypes>,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    fn default() -> Self {
        Self::builder::<Scroll>().build()
    }
}

impl<N, RpcMiddleware> ScrollAddOns<N, RpcMiddleware>
where
    N: FullNodeComponents<Types: ScrollNodeTypes>,
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
        Evm: ConfigureEngineEvm<
            <<N::Types as NodeTypes>::Payload as PayloadTypes>::ExecutionData,
            NextBlockEnvCtx = ScrollNextBlockEnvAttributes,
        >,
    >,
    ScrollEthApiError: FromEvmError<N::Evm>,
    EvmFactoryFor<N::Evm>: EvmFactory<Tx = ScrollTransactionIntoTxEnv<TxEnv>, BlockEnv = BlockEnv>,
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
        Evm: ConfigureEngineEvm<
            <<N::Types as NodeTypes>::Payload as PayloadTypes>::ExecutionData,
            NextBlockEnvCtx = ScrollNextBlockEnvAttributes,
        >,
    >,
    ScrollEthApiError: FromEvmError<N::Evm>,
    EvmFactoryFor<N::Evm>: EvmFactory<Tx = ScrollTransactionIntoTxEnv<TxEnv>, BlockEnv = BlockEnv>,
    RpcMiddleware: RethRpcMiddleware,
{
    type EthApi = <ScrollEthApiBuilder as EthApiBuilder<N>>::EthApi;

    fn hooks_mut(&mut self) -> &mut reth_node_builder::rpc::RpcHooks<N, Self::EthApi> {
        self.rpc_add_ons.hooks_mut()
    }
}

impl<N> EngineValidatorAddOn<N> for ScrollAddOns<N>
where
    N: FullNodeComponents<
        Types: NodeTypes<
            ChainSpec = ScrollChainSpec,
            Primitives = ScrollPrimitives,
            Payload = ScrollEngineTypes,
        >,
        Evm: ConfigureEngineEvm<
            <<N::Types as NodeTypes>::Payload as PayloadTypes>::ExecutionData,
            NextBlockEnvCtx = ScrollNextBlockEnvAttributes,
        >,
    >,
    ScrollEthApiBuilder: EthApiBuilder<N>,
{
    type ValidatorBuilder = BasicEngineValidatorBuilder<ScrollEngineValidatorBuilder>;

    fn engine_validator_builder(&self) -> Self::ValidatorBuilder {
        EngineValidatorAddOn::engine_validator_builder(&self.rpc_add_ons)
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
            min_suggested_priority_fee: DEFAULT_MIN_SUGGESTED_PRIORITY_FEE,
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
        N: FullNodeComponents<Types: ScrollNodeTypes>,
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
                ScrollEngineValidatorBuilder::default(),
                BasicEngineApiBuilder::default(),
                BasicEngineValidatorBuilder::default(),
                rpc_middleware,
            ),
        }
    }
}
