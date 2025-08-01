use super::*;
use alloy_primitives::B256;
use reth_tokio_util::EventStream;
use tokio::sync::oneshot;

/// The message that is broadcast to subscribers of the block import channel.
#[derive(Debug, Clone)]
pub struct NewBlockWithPeer<B> {
    /// The peer that sent the block.
    pub peer_id: PeerId,
    /// The block that was received.
    pub block: B,
}

/// Provides a listener for new blocks on the eth wire protocol.
pub trait EthWireProvider<N: NetworkPrimitives> {
    /// Create a new eth wire block listener.
    fn eth_wire_block_listener(
        &self,
    ) -> impl Future<
        Output = Result<EventStream<NewBlockWithPeer<N::Block>>, oneshot::error::RecvError>,
    > + Send;

    /// Announce a new block to the network over the eth wire protocol.
    fn eth_wire_announce_block(&self, block: N::NewBlockPayload, hash: B256);
}
