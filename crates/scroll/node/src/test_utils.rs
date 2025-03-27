use crate::{ScrollBuiltPayload, ScrollNode as OtherScrollNode, ScrollPayloadBuilderAttributes};
use alloy_genesis::Genesis;
use alloy_primitives::{Address, B256};
use alloy_rpc_types_engine::PayloadAttributes;
use reth_e2e_test_utils::{
    node::NodeTestContext, transaction::TransactionTestContext, wallet::Wallet, NodeBuilderHelper,
    NodeHelperType, TmpDB,
};
use reth_node_api::NodeTypesWithDBAdapter;
use reth_node_builder::{Node, NodeBuilder, NodeConfig, NodeHandle};
use reth_node_core::args::{DiscoveryArgs, NetworkArgs, RpcServerArgs};
use reth_payload_builder::EthPayloadBuilderAttributes;
use reth_provider::providers::BlockchainProvider;
use reth_rpc_server_types::RpcModuleSelection;
use reth_scroll_chainspec::{ScrollChainSpec, ScrollChainSpecBuilder};
use reth_tasks::TaskManager;
use reth_transaction_pool::PeerId;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Optimism Node Helper type
pub(crate) type ScrollNode = NodeHelperType<
    OtherScrollNode,
    BlockchainProvider<NodeTypesWithDBAdapter<OtherScrollNode, TmpDB>>,
>;

pub async fn setup_engine(
    chain_spec: Arc<ScrollChainSpec>,
    is_dev: bool,
) -> eyre::Result<(ScrollNode, TaskManager, PeerId)> {
    // Create a [`TaskManager`] to manage the tasks.
    let tasks = TaskManager::current();
    let exec = tasks.executor();

    // Define the network configuration with discovery disabled.
    let network_config = NetworkArgs {
        discovery: DiscoveryArgs { disable_discovery: true, ..DiscoveryArgs::default() },
        ..NetworkArgs::default()
    };

    // Create the node config
    let node_config = NodeConfig::new(chain_spec.clone())
        .with_network(network_config.clone())
        .with_rpc(RpcServerArgs::default().with_http().with_http_api(RpcModuleSelection::All))
        .set_dev(is_dev);

    // Create the node for a bridge node that will bridge messages from the eth-wire protocol
    // to the scroll-wire protocol.
    let node = OtherScrollNode;
    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .with_types_and_provider::<OtherScrollNode, BlockchainProvider<_>>()
        .with_components(node.components_builder())
        .with_add_ons(node.add_ons())
        .launch()
        .await?;
    let peer_id = *node.network.peer_id();
    let node = NodeTestContext::new(node, scroll_payload_attributes).await?;

    Ok((node, tasks, peer_id))
}

/// Creates the initial setup with `num_nodes` of the node config, started and connected.
pub async fn setup(is_dev: bool) -> eyre::Result<(ScrollNode, TaskManager, Wallet)> {
    let genesis: Genesis =
        serde_json::from_str(include_str!("../tests/assets/genesis.json")).unwrap();
    let chain_spec =
        ScrollChainSpecBuilder::scroll_mainnet().genesis(genesis).build(Default::default());
    let chain_id = chain_spec.chain().into();
    let (node, task_manager, _peer_id) = setup_engine(Arc::new(chain_spec), is_dev).await.unwrap();
    Ok((node, task_manager, Wallet::default().with_chain_id(chain_id)))
}

/// Advance the chain with sequential payloads returning them in the end.
pub async fn advance_chain(
    length: usize,
    node: &mut ScrollNode,
    wallet: Arc<Mutex<Wallet>>,
) -> eyre::Result<Vec<ScrollBuiltPayload>> {
    node.advance(length as u64, |_| {
        let wallet = wallet.clone();
        Box::pin(async move {
            let mut wallet = wallet.lock().await;
            let tx_fut = TransactionTestContext::transfer_tx_nonce_bytes(
                wallet.chain_id,
                wallet.inner.clone(),
                wallet.inner_nonce,
            );
            wallet.inner_nonce += 1;
            tx_fut.await
        })
    })
    .await
}

/// Helper function to create a new eth payload attributes
pub fn scroll_payload_attributes(timestamp: u64) -> ScrollPayloadBuilderAttributes {
    let attributes = PayloadAttributes {
        timestamp,
        prev_randao: B256::ZERO,
        suggested_fee_recipient: Address::ZERO,
        withdrawals: Some(vec![]),
        parent_beacon_block_root: Some(B256::ZERO),
    };

    ScrollPayloadBuilderAttributes {
        payload_attributes: EthPayloadBuilderAttributes::new(B256::ZERO, attributes),
        transactions: vec![],
        no_tx_pool: false,
    }
}
