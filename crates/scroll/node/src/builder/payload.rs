use reth_node_api::{ConfigureEvm, PrimitivesTy};
use reth_node_builder::{components::PayloadBuilderBuilder, BuilderContext, FullNodeTypes};
use reth_node_types::{NodeTypesWithEngine, TxTy};
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_engine_primitives::ScrollEngineTypes;
use reth_scroll_evm::ScrollEvmConfig;
use reth_scroll_payload::ScrollPayloadTransactions;
use reth_scroll_primitives::{ScrollPrimitives, ScrollTransactionSigned};
use reth_transaction_pool::{PoolTransaction, TransactionPool};

/// Payload builder for Scroll.
#[derive(Debug, Clone, Default, Copy)]
pub struct ScrollPayloadBuilder<Txs = ()> {
    /// Returns the current best transactions from the mempool.
    pub best_transactions: Txs,
}

impl<Txs> ScrollPayloadBuilder<Txs> {
    /// A helper method to initialize [`reth_optimism_payload_builder::OpPayloadBuilder`] with the
    /// given EVM config.
    pub fn build<Node, Evm, Pool>(
        self,
        evm_config: Evm,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<reth_scroll_payload::ScrollPayloadBuilder<Pool, Node::Provider, Evm, Txs>>
    where
        Node: FullNodeTypes<
            Types: NodeTypesWithEngine<
                Engine = ScrollEngineTypes,
                ChainSpec = ScrollChainSpec,
                Primitives = ScrollPrimitives,
            >,
        >,
        Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TxTy<Node::Types>>>
            + Unpin
            + 'static,
        Evm: ConfigureEvm<Primitives = PrimitivesTy<Node::Types>>,
        Txs: ScrollPayloadTransactions<Pool::Transaction>,
    {
        let payload_builder = reth_scroll_payload::ScrollPayloadBuilder::new(
            pool,
            evm_config,
            ctx.provider().clone(),
        )
        .with_transactions(self.best_transactions);

        Ok(payload_builder)
    }
}

// impl<Node, Pool, Txs> PayloadServiceBuilder<Node, Pool> for ScrollPayloadBuilder<Txs>
// where
//     Node: FullNodeTypes,
//     Node::Types: NodeTypesWithEngine<
//         Primitives = ScrollPrimitives,
//         Engine = ScrollEngineTypes,
//         ChainSpec = ScrollChainSpec,
//     >,
//     Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TxTy<Node::Types>>>
//         + Unpin
//         + 'static,
//     Txs: ScrollPayloadTransactions<Pool::Transaction>,
// {
//     async fn spawn_payload_builder_service(
//         self,
//         ctx: &BuilderContext<Node>,
//         _pool: Pool,
//     ) -> eyre::Result<PayloadBuilderHandle<<Node::Types as NodeTypesWithEngine>::Engine>> {
//         let payload_builder = reth_scroll_payload::ScrollPayloadBuilder::default();

//         let conf = ctx.config().builder.clone();

//         let payload_job_config = BasicPayloadJobGeneratorConfig::default()
//             .interval(conf.interval)
//             .deadline(conf.deadline)
//             .max_payload_tasks(conf.max_payload_tasks);

//         let payload_generator = BasicPayloadJobGenerator::with_builder(
//             ctx.provider().clone(),
//             ctx.task_executor().clone(),
//             payload_job_config,
//             payload_builder,
//         );
//         let (payload_service, payload_service_handle) =
//             PayloadBuilderService::new(payload_generator,
// ctx.provider().canonical_state_stream());

//         ctx.task_executor().spawn_critical("payload builder service", Box::pin(payload_service));

//         Ok(payload_service_handle)
//     }
// }

impl<Node, Pool, Txs> PayloadBuilderBuilder<Node, Pool> for ScrollPayloadBuilder<Txs>
where
    Node: FullNodeTypes<
        Types: NodeTypesWithEngine<
            Engine = ScrollEngineTypes,
            ChainSpec = ScrollChainSpec,
            Primitives = ScrollPrimitives,
        >,
    >,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = ScrollTransactionSigned>>
        + Unpin
        + 'static,
    Txs: ScrollPayloadTransactions<Pool::Transaction>,
    <Pool as TransactionPool>::Transaction: PoolTransaction<Consensus = ScrollTransactionSigned>,
{
    type PayloadBuilder =
        reth_scroll_payload::ScrollPayloadBuilder<Pool, Node::Provider, ScrollEvmConfig, Txs>;

    async fn build_payload_builder(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<Self::PayloadBuilder> {
        self.build(ScrollEvmConfig::scroll(ctx.chain_spec()), ctx, pool)
    }
}
