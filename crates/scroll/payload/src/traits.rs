use alloy_consensus::{BlockBody, Header};
use reth_primitives_traits::{NodePrimitives, Receipt, SignedTransaction};
use reth_scroll_primitives::transaction::signed::IsL1Message;

/// Helper trait to encapsulate common bounds on [`NodePrimitives`] for Scroll payload builder.
pub trait ScrollPayloadPrimitives:
    NodePrimitives<
    Receipt: Receipt,
    SignedTx = Self::_TX,
    BlockHeader = Header,
    BlockBody = BlockBody<Self::_TX>,
>
{
    /// Helper AT to bound [`NodePrimitives::Block`] type without causing bound cycle.
    type _TX: SignedTransaction + IsL1Message;
}

impl<Tx, T> ScrollPayloadPrimitives for T
where
    Tx: SignedTransaction + IsL1Message,
    T: NodePrimitives<
        SignedTx = Tx,
        Receipt: Receipt,
        BlockHeader = Header,
        BlockBody = BlockBody<Tx>,
    >,
{
    type _TX = Tx;
}
