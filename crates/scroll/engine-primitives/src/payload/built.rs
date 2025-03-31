//! Outcome of a Scroll block building task with payload attributes provided via the Engine API.

use core::iter;
use std::sync::Arc;

use alloy_consensus::Block;
use alloy_eips::eip7685::Requests;
use alloy_primitives::U256;
use alloy_rpc_types_engine::{
    BlobsBundleV1, ExecutionPayloadEnvelopeV2, ExecutionPayloadEnvelopeV3,
    ExecutionPayloadEnvelopeV4, ExecutionPayloadFieldV2, ExecutionPayloadV1, ExecutionPayloadV3,
    PayloadId,
};
use reth_chain_state::ExecutedBlockWithTrieUpdates;
use reth_payload_primitives::BuiltPayload;
use reth_primitives::NodePrimitives;
use reth_primitives_traits::{SealedBlock, SignedTransaction};
use reth_scroll_primitives::ScrollPrimitives;

/// Contains the built payload.
#[derive(Debug, Clone, Default)]
pub struct ScrollBuiltPayload<N: NodePrimitives = ScrollPrimitives> {
    /// Identifier of the payload
    pub(crate) id: PayloadId,
    /// Sealed block
    pub(crate) block: Arc<SealedBlock<N::Block>>,
    /// Block execution data for the payload
    pub(crate) executed_block: Option<ExecutedBlockWithTrieUpdates<N>>,
    /// The fees of the block
    pub(crate) fees: U256,
}

impl<N: NodePrimitives> ScrollBuiltPayload<N> {
    /// Initializes the payload with the given initial block.
    pub const fn new(
        id: PayloadId,
        block: Arc<SealedBlock<N::Block>>,
        executed_block: Option<ExecutedBlockWithTrieUpdates<N>>,
        fees: U256,
    ) -> Self {
        Self { id, block, executed_block, fees }
    }

    /// Returns the identifier of the payload.
    pub const fn id(&self) -> PayloadId {
        self.id
    }

    /// Returns the built block(sealed)
    #[allow(clippy::missing_const_for_fn)]
    pub fn block(&self) -> &SealedBlock<N::Block> {
        &self.block
    }

    /// Fees of the block
    pub const fn fees(&self) -> U256 {
        self.fees
    }

    /// Converts the value into [`SealedBlock`].
    pub fn into_sealed_block(self) -> SealedBlock<N::Block> {
        Arc::unwrap_or_clone(self.block)
    }
}

impl<N: NodePrimitives> BuiltPayload for ScrollBuiltPayload<N> {
    type Primitives = N;

    fn block(&self) -> &SealedBlock<N::Block> {
        self.block()
    }

    fn fees(&self) -> U256 {
        self.fees
    }

    fn executed_block(&self) -> Option<ExecutedBlockWithTrieUpdates<N>> {
        self.executed_block.clone()
    }

    fn requests(&self) -> Option<Requests> {
        None
    }
}

// V1 engine_getPayloadV1 response
impl<T, N> From<ScrollBuiltPayload<N>> for ExecutionPayloadV1
where
    T: SignedTransaction,
    N: NodePrimitives<Block = Block<T>>,
{
    fn from(value: ScrollBuiltPayload<N>) -> Self {
        Self::from_block_unchecked(
            value.block().hash(),
            &Arc::unwrap_or_clone(value.block).into_block(),
        )
    }
}

// V2 engine_getPayloadV2 response
impl<T, N> From<ScrollBuiltPayload<N>> for ExecutionPayloadEnvelopeV2
where
    T: SignedTransaction,
    N: NodePrimitives<Block = Block<T>>,
{
    fn from(value: ScrollBuiltPayload<N>) -> Self {
        let ScrollBuiltPayload { block, fees, .. } = value;

        Self {
            block_value: fees,
            execution_payload: ExecutionPayloadFieldV2::from_block_unchecked(
                block.hash(),
                &Arc::unwrap_or_clone(block).into_block(),
            ),
        }
    }
}

impl<T, N> From<ScrollBuiltPayload<N>> for ExecutionPayloadEnvelopeV3
where
    T: SignedTransaction,
    N: NodePrimitives<Block = Block<T>>,
{
    fn from(value: ScrollBuiltPayload<N>) -> Self {
        let ScrollBuiltPayload { block, fees, .. } = value;

        Self {
            execution_payload: ExecutionPayloadV3::from_block_unchecked(
                block.hash(),
                &Arc::unwrap_or_clone(block).into_block(),
            ),
            block_value: fees,
            // From the engine API spec:
            //
            // > Client software **MAY** use any heuristics to decide whether to set
            // `shouldOverrideBuilder` flag or not. If client software does not implement any
            // heuristic this flag **SHOULD** be set to `false`.
            //
            // Spec:
            // <https://github.com/ethereum/execution-apis/blob/fe8e13c288c592ec154ce25c534e26cb7ce0530d/src/engine/cancun.md#specification-2>
            should_override_builder: false,
            blobs_bundle: BlobsBundleV1::new(iter::empty()),
        }
    }
}
impl<T, N> From<ScrollBuiltPayload<N>> for ExecutionPayloadEnvelopeV4
where
    T: SignedTransaction,
    N: NodePrimitives<Block = Block<T>>,
{
    fn from(value: ScrollBuiltPayload<N>) -> Self {
        Self { envelope_inner: value.into(), execution_requests: Default::default() }
    }
}
