use reth_eth_wire_types::BasicNetworkPrimitives;
use reth_network::{
    config::NetworkMode, transform::header::HeaderTransform, NetworkConfig, NetworkHandle,
    NetworkManager, PeersInfo,
};
use reth_node_api::TxTy;
use reth_node_builder::{components::NetworkBuilder, BuilderContext, FullNodeTypes};
use reth_node_types::NodeTypes;
use reth_primitives_traits::BlockHeader;
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_primitives::ScrollPrimitives;
use reth_tracing::tracing::info;
use reth_transaction_pool::{PoolTransaction, TransactionPool};
use scroll_alloy_hardforks::ScrollHardforks;
use std::fmt::Debug;

/// The network builder for Scroll.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScrollNetworkBuilder {
    /// Disable transaction pool broadcast
    pub disable_txpool_broadcast: bool,
    /// Disable transaction pool receive
    pub disable_txpool_receive: bool,
}

impl ScrollNetworkBuilder {
    /// Returns the [`NetworkConfig`] that contains the settings to launch the p2p network.
    ///
    /// This applies the configured [`ScrollNetworkBuilder`] settings.
    pub fn network_config<Node>(
        &self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<NetworkConfig<<Node as FullNodeTypes>::Provider, ScrollNetworkPrimitives>>
    where
        Node:
            FullNodeTypes<Types: NodeTypes<ChainSpec = ScrollChainSpec, Primitives = ScrollPrimitives>>,
    {
        let Self { disable_txpool_broadcast, disable_txpool_receive } = self.clone();
        // set the network mode to work.
        let config = ctx.network_config()?;

        let network_config = NetworkConfig {
            network_mode: NetworkMode::Work,
            header_transform: Box::new(transform),
            // When `sequencer_endpoint` is configured, the node will forward all transactions to a
            // Sequencer node for execution and inclusion on L1, and disable its own txpool
            // gossip broadcast/receive to prevent other parties in the network from learning about them.
            tx_gossip_broadcast_disabled: disable_txpool_broadcast,
            tx_gossip_receive_disabled: disable_txpool_receive,
            ..config
        };

        Ok(network_config)
    }
}

impl<Node, Pool> NetworkBuilder<Node, Pool> for ScrollNetworkBuilder
where
    Node:
        FullNodeTypes<Types: NodeTypes<ChainSpec = ScrollChainSpec, Primitives = ScrollPrimitives>>,
    Pool: TransactionPool<
            Transaction: PoolTransaction<
                Consensus = TxTy<Node::Types>,
                Pooled = scroll_alloy_consensus::ScrollPooledTransaction,
            >,
        > + Unpin
        + 'static,
{
    type Network = NetworkHandle<ScrollNetworkPrimitives>;

    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<Self::Network> {
        // get the header transform.
        let chain_spec = ctx.chain_spec();
        let transform = ScrollHeaderTransform { chain_spec };

        let config = self::network_config(ctx);

        let network = NetworkManager::builder(config).await?;
        let handle = ctx.start_network(network, pool);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

/// Network primitive types used by Scroll networks.
pub type ScrollNetworkPrimitives =
    BasicNetworkPrimitives<ScrollPrimitives, scroll_alloy_consensus::ScrollPooledTransaction>;

/// An implementation of a [`HeaderTransform`] for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollHeaderTransform<ChainSpec> {
    chain_spec: ChainSpec,
}

impl<ChainSpec: ScrollHardforks + Debug + Send + Sync + 'static> ScrollHeaderTransform<ChainSpec> {
    /// Returns a new instance of the [`ScrollHeaderTransform`] from the provider chain spec.
    pub const fn new(chain_spec: ChainSpec) -> Self {
        Self { chain_spec }
    }

    /// Returns a new [`ScrollHeaderTransform`] as a [`HeaderTransform`] trait object.
    pub fn boxed<H: BlockHeader>(chain_spec: ChainSpec) -> Box<dyn HeaderTransform<H>> {
        Box::new(Self { chain_spec })
    }
}

impl<H: BlockHeader, ChainSpec: ScrollHardforks + Debug + Send + Sync> HeaderTransform<H>
    for ScrollHeaderTransform<ChainSpec>
{
    fn map(&self, mut header: H) -> H {
        if self.chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
            // clear the extra data field.
            *header.extra_data_mut() = Default::default()
        }
        header
    }
}
