//! Node specific implementations for Scroll.

use reth_chainspec::ChainSpec;
use reth_consensus::noop::NoopConsensus;
use reth_db::transaction::{DbTx, DbTxMut};
use reth_ethereum_engine_primitives::EthEngineTypes;
use reth_ethereum_forks::EthereumHardforks;
use reth_evm::execute::BasicBlockExecutorProvider;
use reth_network::PeersInfo;
use reth_node_builder::{
    components::{
        Components, ComponentsBuilder, ConsensusBuilder, ExecutorBuilder, NetworkBuilder,
        PayloadServiceBuilder, PoolBuilder,
    },
    BuilderContext, FullNodeTypes, Node,
};
use reth_node_types::{NodePrimitives, NodeTypes, NodeTypesWithEngine};
use reth_payload_builder::PayloadBuilderHandle;
use reth_primitives::BlockBody;
use reth_provider::{
    providers::ChainStorage, BlockBodyReader, BlockBodyWriter, ChainSpecProvider, DBProvider,
    EthStorage, ProviderResult, ReadBodyInput,
};
use reth_scroll_evm::{ScrollEvmConfig, ScrollExecutionStrategyFactory};
use reth_tracing::tracing::info;
use reth_transaction_pool::{noop::NoopTransactionPool, TransactionPool};
use reth_trie_db::MerklePatriciaTrie;

// #################### NODE ####################

/// The Scroll node implementation.
#[derive(Clone, Debug)]
pub struct ScrollNode;

impl<N> Node<N> for ScrollNode
where
    N: FullNodeTypes,
    // TODO (scroll): replace with `ScrollChainSpec`.
    N::Types: NodeTypes<ChainSpec = ChainSpec, Primitives = ScrollPrimitives>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        ScrollPoolBuilder,
        ScrollPayloadBuilder,
        ScrollNetworkBuilder,
        ScrollExecutorBuilder,
        ScrollConsensusBuilder,
    >;
    type AddOns = ();

    fn components_builder(&self) -> Self::ComponentsBuilder {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(ScrollPoolBuilder)
            .payload(ScrollPayloadBuilder)
            .network(ScrollNetworkBuilder)
            .executor(ScrollExecutorBuilder)
            .consensus(ScrollConsensusBuilder)
    }

    fn add_ons(&self) -> Self::AddOns {}
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
    Pool: TransactionPool,
{
    async fn spawn_payload_service(
        self,
        _ctx: &BuilderContext<Node>,
        _pool: Pool,
    ) -> eyre::Result<
        PayloadBuilderHandle<<<Node as FullNodeTypes>::Types as NodeTypesWithEngine>::Engine>,
    > {
        // TODO (scroll): trying to send on the channel will return an error.
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        eyre::Ok(PayloadBuilderHandle::new(tx))
    }
}

// #################### NODE NETWORK ####################

/// The network builder for Scroll.
#[derive(Debug)]
pub struct ScrollNetworkBuilder;

impl<Node, Pool> NetworkBuilder<Node, Pool> for ScrollNetworkBuilder
where
    // TODO (scroll): replace with
    // ScrollChainSpec.
    Node: FullNodeTypes,
    Node::Types: NodeTypes<ChainSpec = ChainSpec, Primitives = ScrollPrimitives>,
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
    // TODO (scroll): replace with
    // ScrollChainSpec.
    Node: FullNodeTypes,
    Node::Types: NodeTypesWithEngine<ChainSpec = ChainSpec>,
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

impl ChainStorage<ScrollPrimitives> for ScrollStorage {
    fn reader<TX, Types>(
        &self,
    ) -> impl reth_provider::ChainStorageReader<
        reth_provider::DatabaseProvider<TX, Types>,
        ScrollPrimitives,
    >
    where
        TX: DbTx + 'static,
        Types: reth_provider::providers::NodeTypesForProvider<Primitives = ScrollPrimitives>,
    {
        self
    }

    fn writer<TX, Types>(
        &self,
    ) -> impl reth_provider::ChainStorageWriter<
        reth_provider::DatabaseProvider<TX, Types>,
        ScrollPrimitives,
    >
    where
        TX: DbTxMut + DbTx + 'static,
        Types: NodeTypes<Primitives = ScrollPrimitives>,
    {
        self
    }
}

// #################### NODE TYPES ####################

impl NodeTypesWithEngine for ScrollNode {
    type Engine = EthEngineTypes;
}

impl NodeTypes for ScrollNode {
    type Primitives = ScrollPrimitives;
    // TODO (scroll): replace with ScrollChainSpec.
    type ChainSpec = ChainSpec;
    // TODO (scroll): replace with BinaryMerklePatriciaTrie.
    type StateCommitment = MerklePatriciaTrie;
    type Storage = ScrollStorage;
}

/// The primitive types for Scroll.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScrollPrimitives;

impl NodePrimitives for ScrollPrimitives {
    type Block = reth_primitives::Block;
    type BlockHeader = reth_primitives::Header;
    type BlockBody = BlockBody;
    type SignedTx = reth_primitives::TransactionSigned;
    type TxType = reth_primitives::TxType;
    type Receipt = reth_primitives::Receipt;
}
