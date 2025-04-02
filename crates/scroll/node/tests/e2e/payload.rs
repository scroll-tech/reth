use reth_scroll_node::test_utils::{advance_chain, setup};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn can_sync() -> eyre::Result<()> {
    reth_tracing::init_test_tracing();

    let (mut node, _tasks, wallet) = setup(3, false).await?;
    let mut node = node.pop().unwrap();
    let wallet = Arc::new(Mutex::new(wallet));

    let tip: usize = 90;

    // On first node, create a chain up to block number 90a
    let canonical_payload_chain = advance_chain(tip, &mut node, wallet.clone()).await?;

    Ok(())
}
