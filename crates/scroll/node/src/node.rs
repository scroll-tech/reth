//! Node specific implementations for Scroll.

use crate::{
    args::ScrollRollupArgs, ScrollAddOns, ScrollAddOnsBuilder, ScrollConsensusBuilder,
    ScrollExecutorBuilder, ScrollNetworkBuilder, ScrollPayloadBuilderBuilder, ScrollPoolBuilder,
    ScrollStorage,
};
use reth_engine_local::LocalPayloadAttributesBuilder;
use reth_node_api::{FullNodeComponents, PayloadAttributesBuilder, PayloadTypes};
use reth_node_builder::{
    components::{BasicPayloadServiceBuilder, ComponentsBuilder},
    node::{FullNodeTypes, NodeTypes},
    DebugNode, Node, NodeAdapter, NodeComponentsBuilder,
};
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_engine_primitives::ScrollEngineTypes;
use reth_scroll_primitives::ScrollPrimitives;
use reth_trie_db::MerklePatriciaTrie;
use scroll_alloy_network::Scroll;
use std::sync::Arc;

/// The Scroll node implementation.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ScrollNode {
    /// Additional Scroll args.
    pub args: ScrollRollupArgs,
}

impl ScrollNode {
    /// Creates a new instance of the Scroll node type.
    pub const fn new(args: ScrollRollupArgs) -> Self {
        Self { args }
    }

    /// Returns a [`ComponentsBuilder`] configured for a regular Ethereum node.
    pub fn components<Node>() -> ComponentsBuilder<
        Node,
        ScrollPoolBuilder,
        BasicPayloadServiceBuilder<ScrollPayloadBuilderBuilder>,
        ScrollNetworkBuilder,
        ScrollExecutorBuilder,
        ScrollConsensusBuilder,
    >
    where
        Node: FullNodeTypes<
            Types: NodeTypes<
                ChainSpec = ScrollChainSpec,
                Primitives = ScrollPrimitives,
                Payload = ScrollEngineTypes,
            >,
        >,
    {
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(ScrollPoolBuilder::default())
            .executor(ScrollExecutorBuilder::default())
            .payload(BasicPayloadServiceBuilder::new(ScrollPayloadBuilderBuilder::default()))
            .network(ScrollNetworkBuilder::new())
            .executor(ScrollExecutorBuilder)
            .consensus(ScrollConsensusBuilder)
    }
}

impl<N> Node<N> for ScrollNode
where
    N: FullNodeTypes<Types = Self>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        ScrollPoolBuilder,
        BasicPayloadServiceBuilder<ScrollPayloadBuilderBuilder>,
        ScrollNetworkBuilder,
        ScrollExecutorBuilder,
        ScrollConsensusBuilder,
    >;

    type AddOns = ScrollAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }

    fn add_ons(&self) -> Self::AddOns {
        ScrollAddOnsBuilder::<Scroll, _>::default()
            .with_sequencer(self.args.sequencer.clone())
            .with_min_suggested_priority_fee(self.args.min_suggested_priority_fee)
            .with_payload_size_limit(self.args.payload_size_limit)
            .build()
    }
}

impl<N> DebugNode<N> for ScrollNode
where
    N: FullNodeComponents<Types = Self>,
{
    type RpcBlock = alloy_rpc_types_eth::Block<scroll_alloy_consensus::ScrollTxEnvelope>;

    fn rpc_to_primitive_block(rpc_block: Self::RpcBlock) -> reth_node_api::BlockTy<Self> {
        rpc_block.into_consensus()
    }

    fn local_payload_attributes_builder(
        chain_spec: &Self::ChainSpec,
    ) -> impl PayloadAttributesBuilder<<<Self as NodeTypes>::Payload as PayloadTypes>::PayloadAttributes>
    {
        LocalPayloadAttributesBuilder::new(Arc::new(chain_spec.clone()))
    }
}

impl NodeTypes for ScrollNode {
    type Primitives = ScrollPrimitives;
    type ChainSpec = ScrollChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = ScrollStorage;
    type Payload = ScrollEngineTypes;
}
