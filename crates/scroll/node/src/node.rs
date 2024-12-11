//! Node specific implementations for Scroll.

use alloy_rpc_types_engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError};
use reth_consensus::noop::NoopConsensus;
use reth_db::transaction::{DbTx, DbTxMut};
use reth_ethereum_engine_primitives::{
    EthBuiltPayload, EthEngineTypes, EthPayloadAttributes, EthPayloadBuilderAttributes,
};
use reth_ethereum_forks::EthereumHardforks;
use reth_evm::execute::BasicBlockExecutorProvider;
use reth_network::{NetworkHandle, PeersInfo};
use reth_node_builder::{
    components::{
        ComponentsBuilder, ConsensusBuilder, ExecutorBuilder, NetworkBuilder,
        PayloadServiceBuilder, PoolBuilder,
    },
    rpc::{EngineValidatorBuilder, RpcAddOns},
    AddOnsContext, BuilderContext, EngineApiMessageVersion, EngineObjectValidationError,
    EngineTypes, EngineValidator, FullNodeComponents, FullNodeTypes, Node, NodeAdapter,
    NodeComponentsBuilder, PayloadOrAttributes, PayloadTypes,
};
use reth_node_types::{NodeTypes, NodeTypesWithDB, NodeTypesWithEngine};
use reth_payload_builder::{
    test_utils::TestPayloadJobGenerator, PayloadBuilderHandle, PayloadBuilderService,
};
use reth_primitives::{Block, BlockBody, EthPrimitives, SealedBlock};
use reth_provider::{
    providers::ChainStorage, BlockBodyReader, BlockBodyWriter, CanonStateSubscriptions,
    ChainSpecProvider, DBProvider, EthStorage, ProviderResult, ReadBodyInput,
};
use reth_rpc::EthApi;
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_evm::{ScrollEvmConfig, ScrollExecutionStrategyFactory};
use reth_scroll_state_commitment::BinaryMerklePatriciaTrie;
use reth_tracing::tracing::info;
use reth_transaction_pool::{noop::NoopTransactionPool, TransactionPool};

// #################### NODE ####################

/// The Scroll node implementation.
#[derive(Clone, Debug)]
pub struct ScrollNode;

impl<N> Node<N> for ScrollNode
where
    N: FullNodeTypes,
    N::Types: NodeTypesWithDB
        + NodeTypesWithEngine<
            ChainSpec = ScrollChainSpec,
            Primitives = EthPrimitives,
            Engine = EthEngineTypes,
            Storage = ScrollStorage,
        >,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        ScrollPoolBuilder,
        ScrollPayloadBuilder,
        ScrollNetworkBuilder,
        ScrollExecutorBuilder,
        ScrollConsensusBuilder,
    >;

    type AddOns = ScrollAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(ScrollPoolBuilder)
            .payload(ScrollPayloadBuilder)
            .network(ScrollNetworkBuilder)
            .executor(ScrollExecutorBuilder)
            .consensus(ScrollConsensusBuilder)
    }

    fn add_ons(&self) -> Self::AddOns {
        ScrollAddOns::default()
    }
}

// #################### NODE TYPES ####################

impl NodeTypesWithEngine for ScrollNode {
    type Engine = EthEngineTypes;
}

impl NodeTypes for ScrollNode {
    type Primitives = EthPrimitives;
    type ChainSpec = ScrollChainSpec;
    type StateCommitment = BinaryMerklePatriciaTrie;
    type Storage = ScrollStorage;
}

// #################### NODE ADD-ONS ####################

/// Add-ons for the Scroll follower node.
pub type ScrollAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    ScrollEngineValidatorBuilder,
>;

/// Builder for [`ScrollEngineValidator`].
#[derive(Debug, Default, Clone)]
pub struct ScrollEngineValidatorBuilder;

impl<Node, Types> EngineValidatorBuilder<Node> for ScrollEngineValidatorBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ScrollChainSpec>,
    Node: FullNodeComponents<Types = Types>,
    NoopEngineValidator: EngineValidator<Types::Engine>,
{
    type Validator = NoopEngineValidator;

    async fn build(self, _ctx: &AddOnsContext<'_, Node>) -> eyre::Result<Self::Validator> {
        Ok(NoopEngineValidator)
    }
}

/// Noop engine validator used as default for Scroll.
#[derive(Debug, Clone)]
pub struct NoopEngineValidator;

impl<Types> EngineValidator<Types> for NoopEngineValidator
where
    Types: EngineTypes<PayloadAttributes = EthPayloadAttributes>,
{
    type Block = Block;

    fn validate_version_specific_fields(
        &self,
        _version: EngineApiMessageVersion,
        _payload_or_attrs: PayloadOrAttributes<'_, EthPayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        Ok(())
    }

    fn ensure_well_formed_attributes(
        &self,
        _version: EngineApiMessageVersion,
        _attributes: &EthPayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        Ok(())
    }

    fn ensure_well_formed_payload(
        &self,
        _payload: ExecutionPayload,
        _sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlock, PayloadError> {
        Ok(SealedBlock::default())
    }
}

// #################### NODE POOL ####################

/// Pool builder for Scroll.
#[derive(Debug)]
pub struct ScrollPoolBuilder;

impl<Node> PoolBuilder<Node> for ScrollPoolBuilder
where
    Node: FullNodeTypes,
{
    type Pool = NoopTransactionPool;

    async fn build_pool(self, _ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        Ok(NoopTransactionPool::default())
    }
}

// #################### NODE PAYLOAD ####################

/// Payload builder for Scroll.
#[derive(Debug)]
pub struct ScrollPayloadBuilder;

impl<Node, Pool> PayloadServiceBuilder<Node, Pool> for ScrollPayloadBuilder
where
    Node: FullNodeTypes,
    Node::Types: NodeTypesWithEngine<Primitives = EthPrimitives>,
    <Node::Types as NodeTypesWithEngine>::Engine: PayloadTypes<
        BuiltPayload = EthBuiltPayload,
        PayloadAttributes = EthPayloadAttributes,
        PayloadBuilderAttributes = EthPayloadBuilderAttributes,
    >,
    Pool: TransactionPool,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        _pool: Pool,
    ) -> eyre::Result<
        PayloadBuilderHandle<<<Node as FullNodeTypes>::Types as NodeTypesWithEngine>::Engine>,
    > {
        let test_payload_generator = TestPayloadJobGenerator::default();
        let (payload_service, payload_builder) = PayloadBuilderService::new(
            test_payload_generator,
            ctx.provider().canonical_state_stream(),
        );

        ctx.task_executor().spawn_critical("payload builder service", Box::pin(payload_service));

        eyre::Ok(payload_builder)
    }
}

// #################### NODE NETWORK ####################

/// The network builder for Scroll.
#[derive(Debug)]
pub struct ScrollNetworkBuilder;

impl<Node, Pool> NetworkBuilder<Node, Pool> for ScrollNetworkBuilder
where
    Node: FullNodeTypes,
    Node::Types: NodeTypes<ChainSpec = ScrollChainSpec, Primitives = EthPrimitives>,
    Pool: TransactionPool + Unpin + 'static,
{
    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<reth_network::NetworkHandle> {
        let network = ctx.network_builder().await?;
        let handle = ctx.start_network(network, pool);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

// #################### NODE EXECUTION ####################

/// Executor builder for Scroll.
#[derive(Debug)]
pub struct ScrollExecutorBuilder;

impl<Node> ExecutorBuilder<Node> for ScrollExecutorBuilder
where
    Node: FullNodeTypes,
    Node::Types: NodeTypesWithEngine<ChainSpec = ScrollChainSpec>,
{
    type EVM = ScrollEvmConfig;
    type Executor = BasicBlockExecutorProvider<ScrollExecutionStrategyFactory>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let chain_spec = ctx.chain_spec();
        let strategy_factory = ScrollExecutionStrategyFactory::new(chain_spec);
        let evm_config = strategy_factory.evm_config();

        let executor = BasicBlockExecutorProvider::new(strategy_factory);

        Ok((evm_config, executor))
    }
}

// #################### NODE CONSENSUS ####################

/// The consensus builder for Scroll.
#[derive(Debug)]
pub struct ScrollConsensusBuilder;

impl<Node: FullNodeTypes> ConsensusBuilder<Node> for ScrollConsensusBuilder {
    type Consensus = NoopConsensus;

    async fn build_consensus(self, _ctx: &BuilderContext<Node>) -> eyre::Result<Self::Consensus> {
        Ok(NoopConsensus::default())
    }
}

// #################### NODE STORAGE ####################

/// Storage implementation for Scroll.
#[derive(Debug, Default, Clone)]
pub struct ScrollStorage(EthStorage);

impl<Provider> BlockBodyWriter<Provider, BlockBody> for ScrollStorage
where
    Provider: DBProvider<Tx: DbTxMut>,
{
    fn write_block_bodies(
        &self,
        provider: &Provider,
        bodies: Vec<(u64, Option<BlockBody>)>,
    ) -> ProviderResult<()> {
        self.0.write_block_bodies(provider, bodies)
    }

    fn remove_block_bodies_above(
        &self,
        provider: &Provider,
        block: alloy_primitives::BlockNumber,
    ) -> ProviderResult<()> {
        self.0.remove_block_bodies_above(provider, block)
    }
}

impl<Provider> BlockBodyReader<Provider> for ScrollStorage
where
    Provider: DBProvider + ChainSpecProvider<ChainSpec: EthereumHardforks>,
{
    type Block = reth_primitives::Block;

    fn read_block_bodies(
        &self,
        provider: &Provider,
        inputs: Vec<ReadBodyInput<'_, Self::Block>>,
    ) -> ProviderResult<Vec<BlockBody>> {
        self.0.read_block_bodies(provider, inputs)
    }
}

impl ChainStorage<EthPrimitives> for ScrollStorage {
    fn reader<TX, Types>(
        &self,
    ) -> impl reth_provider::ChainStorageReader<reth_provider::DatabaseProvider<TX, Types>, EthPrimitives>
    where
        TX: DbTx + 'static,
        Types: reth_provider::providers::NodeTypesForProvider<Primitives = EthPrimitives>,
    {
        self
    }

    fn writer<TX, Types>(
        &self,
    ) -> impl reth_provider::ChainStorageWriter<reth_provider::DatabaseProvider<TX, Types>, EthPrimitives>
    where
        TX: DbTxMut + DbTx + 'static,
        Types: NodeTypes<Primitives = EthPrimitives>,
    {
        self
    }
}
