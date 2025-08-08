use crate::error::ScrollConsensusError;
use alloc::sync::Arc;

use crate::{CLIQUE_IN_TURN_DIFFICULTY, CLIQUE_NO_TURN_DIFFICULTY};
use alloy_consensus::{BlockHeader as _, TxReceipt, EMPTY_OMMER_ROOT_HASH};
use alloy_primitives::{b64, Address, B256, B64, U256};
use core::fmt::Debug;
use reth_chainspec::{EthChainSpec, EthereumHardforks};
use reth_consensus::{
    validate_state_root, Consensus, ConsensusError, FullConsensus, HeaderValidator,
};
use reth_consensus_common::validation::{
    validate_against_parent_hash_number, validate_body_against_header, validate_header_gas,
};
use reth_execution_types::BlockExecutionResult;
use reth_primitives_traits::{
    receipt::gas_spent_by_transactions, Block, BlockBody, BlockHeader, GotExpected, NodePrimitives,
    RecoveredBlock, SealedBlock, SealedHeader, SignedTransaction,
};
use reth_scroll_primitives::ScrollReceipt;
use scroll_alloy_consensus::ScrollTransaction;
use scroll_alloy_hardforks::{ScrollHardfork, ScrollHardforks};

/// Scroll consensus implementation.
///
/// Provides basic checks as outlined in the execution specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollBeaconConsensus<ChainSpec> {
    /// Configuration
    chain_spec: Arc<ChainSpec>,
}

impl<ChainSpec> ScrollBeaconConsensus<ChainSpec> {
    /// Create a new instance of [`ScrollBeaconConsensus`]
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { chain_spec }
    }
}

impl<
        ChainSpec: EthChainSpec + ScrollHardforks,
        N: NodePrimitives<Receipt = ScrollReceipt, SignedTx: ScrollTransaction>,
    > FullConsensus<N> for ScrollBeaconConsensus<ChainSpec>
{
    fn validate_block_post_execution(
        &self,
        block: &RecoveredBlock<N::Block>,
        result: &BlockExecutionResult<N::Receipt>,
    ) -> Result<(), ConsensusError> {
        // verify the block gas used
        let cumulative_gas_used =
            result.receipts.last().map(|r| r.cumulative_gas_used()).unwrap_or(0);
        if block.gas_used() != cumulative_gas_used {
            return Err(ConsensusError::BlockGasUsed {
                gas: GotExpected { got: cumulative_gas_used, expected: block.gas_used() },
                gas_spent_by_tx: gas_spent_by_transactions(&result.receipts),
            });
        }

        // verify the receipts logs bloom and root
        if self.chain_spec.is_byzantium_active_at_block(block.header().number()) {
            if let Err(error) = reth_ethereum_consensus::verify_receipts(
                block.header().receipts_root(),
                block.header().logs_bloom(),
                &result.receipts,
            ) {
                tracing::debug!(
                    %error,
                    ?result.receipts,
                    header_receipt_root = ?block.header().receipts_root(),
                    header_bloom = ?block.header().logs_bloom(),
                    "failed to verify receipts"
                );
                return Err(error);
            }
        }

        Ok(())
    }
}

/// Following fields should be checked on body:
/// - Verify no ommers are present and hash to the header ommer root.
/// - Verify transactions trie root is valid.
/// - Validate L1 messages: should be at the start of the list of transactions and been continuous
///   in regard to the queue index.
impl<ChainSpec, B> Consensus<B> for ScrollBeaconConsensus<ChainSpec>
where
    B: Block,
    <B::Body as BlockBody>::Transaction: ScrollTransaction,
    ChainSpec: EthChainSpec + ScrollHardforks,
{
    type Error = ConsensusError;

    fn validate_body_against_header(
        &self,
        body: &B::Body,
        header: &SealedHeader<B::Header>,
    ) -> Result<(), ConsensusError> {
        validate_body_against_header(body, header.header())
    }

    fn validate_block_pre_execution(&self, block: &SealedBlock<B>) -> Result<(), ConsensusError> {
        // Check no ommers.
        let ommers_len = block.body().ommers().map(|o| o.len()).unwrap_or_default();
        if ommers_len > 0 {
            return Err(ConsensusError::Other("uncles not allowed".to_string()))
        }

        // Check ommers hash
        let ommers_hash = block.body().calculate_ommers_root();
        if Some(block.ommers_hash()) != ommers_hash {
            return Err(ConsensusError::BodyOmmersHashDiff(
                GotExpected {
                    got: ommers_hash.unwrap_or(EMPTY_OMMER_ROOT_HASH),
                    expected: block.ommers_hash(),
                }
                .into(),
            ))
        }

        // Check transaction root
        if let Err(error) = block.ensure_transaction_root_valid() {
            return Err(ConsensusError::BodyTransactionRootDiff(error.into()))
        }

        // Check withdrawals are empty
        if block.body().withdrawals().is_some() {
            return Err(ConsensusError::Other(ScrollConsensusError::WithdrawalsNonEmpty.to_string()))
        }

        // Check L1 messages.
        validate_l1_messages(block.body().transactions())?;

        Ok(())
    }
}

impl<ChainSpec: EthChainSpec + ScrollHardforks, H: BlockHeader> HeaderValidator<H>
    for ScrollBeaconConsensus<ChainSpec>
{
    fn validate_header(&self, header: &SealedHeader<H>) -> Result<(), ConsensusError> {
        if header.ommers_hash() != EMPTY_OMMER_ROOT_HASH {
            return Err(ConsensusError::TheMergeOmmerRootIsNotEmpty)
        }

        validate_header_gas(header.header())?;
        validate_header_base_fee(header.header(), &self.chain_spec)
    }

    fn validate_header_against_parent(
        &self,
        header: &SealedHeader<H>,
        parent: &SealedHeader<H>,
    ) -> Result<(), ConsensusError> {
        validate_against_parent_hash_number(header.header(), parent)?;
        validate_against_parent_timestamp(header.header(), parent.header())?;

        // TODO(scroll): we should have a way to validate the base fee from the header
        // against the parent header using
        // <https://github.com/scroll-tech/go-ethereum/blob/develop/consensus/misc/eip1559.go#L53>

        // ensure that the blob gas fields for this block
        if self.chain_spec.blob_params_at_timestamp(header.timestamp()).is_some() {
            return Err(ConsensusError::Other(
                ScrollConsensusError::UnexpectedBlobParams.to_string(),
            ))
        }

        Ok(())
    }

    fn validate_state_root(&self, header: &H, root: B256) -> Result<(), ConsensusError> {
        if self.chain_spec.is_euclid_active_at_timestamp(header.timestamp()) {
            validate_state_root(header, root)?;
        }

        Ok(())
    }
}

/// Validates the timestamp of the header, which should not be in the future.
#[inline]
fn validate_header_timestamp<H: BlockHeader>(header: &H) -> Result<(), ConsensusError> {
    let now = std::time::SystemTime::now();
    let since_unix_epoch = now.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
    if header.timestamp() > since_unix_epoch {
        return Err(ConsensusError::TimestampIsInPast {
            parent_timestamp: since_unix_epoch,
            timestamp: header.timestamp(),
        })
    }
    Ok(())
}

/// Ensure the EIP-1559 base fee is set if the Curie hardfork is active.
#[inline]
fn validate_header_base_fee<H: BlockHeader, ChainSpec: ScrollHardforks>(
    header: &H,
    chain_spec: &ChainSpec,
) -> Result<(), ConsensusError> {
    if chain_spec.scroll_fork_activation(ScrollHardfork::Curie).active_at_block(header.number()) &&
        header.base_fee_per_gas().is_none()
    {
        return Err(ConsensusError::BaseFeeMissing)
    }
    Ok(())
}

/// Validates the timestamp against the parent to make sure it is in the past.
/// In Scroll, we can have parent.timestamp == header.timestamp which is why
/// we modify this validation compared to
/// [`reth_consensus_common::validation::validate_against_parent_timestamp`].
#[inline]
fn validate_against_parent_timestamp<H: BlockHeader>(
    header: &H,
    parent: &H,
) -> Result<(), ConsensusError> {
    if header.timestamp() < parent.timestamp() {
        return Err(ConsensusError::TimestampIsInPast {
            parent_timestamp: parent.timestamp(),
            timestamp: header.timestamp(),
        })
    }
    Ok(())
}

/// Validate the L1 messages by checking they are only present that the start of the block and only
/// have increasing queue index.
#[inline]
fn validate_l1_messages<Tx: SignedTransaction + ScrollTransaction>(
    txs: &[Tx],
) -> Result<(), ScrollConsensusError> {
    // Check if the block contains L1 messages.
    if !txs.iter().any(ScrollTransaction::is_l1_message) {
        return Ok(())
    }

    // Check L1 messages are only at the start of the block and correctly ordered.
    let mut saw_l2_transaction = false;
    let mut queue_index = 0;

    for tx in txs {
        // Check index is strictly increasing.
        if tx.is_l1_message() {
            let tx_queue_index = tx.queue_index().expect("is_l1_message");
            if tx_queue_index < queue_index {
                return Err(ScrollConsensusError::InvalidL1MessageOrder);
            }
            queue_index = tx_queue_index + 1;
        }

        // Check correct ordering.
        if tx.is_l1_message() && saw_l2_transaction {
            return Err(ScrollConsensusError::InvalidL1MessageOrder);
        }
        saw_l2_transaction = !tx.is_l1_message();
    }

    Ok(())
}
