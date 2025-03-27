use futures::StreamExt;
use reth_scroll_node::test_utils::{advance_chain, setup};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn can_sync() -> eyre::Result<()> {
    reth_tracing::init_test_tracing();

    let (node, _tasks, wallet) = setup(true).await?;
    let wallet = Arc::new(Mutex::new(wallet));

    let tip: usize = 90;
    let tip_index: usize = tip - 1;
    let reorg_depth = 2;

    // On first node, create a chain up to block number 90a
    let canonical_payload_chain = advance_chain(tip, &mut node, wallet.clone()).await?;
    let canonical_chain =
        canonical_payload_chain.iter().map(|p| p.block().hash()).collect::<Vec<_>>();

    Ok(())
}
