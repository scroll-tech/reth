use crate::{
    constants::SCROLL_MAXIMUM_BASE_FEE, error::ScrollConsensusError, CLIQUE_IN_TURN_DIFFICULTY,
    CLIQUE_NO_TURN_DIFFICULTY,
};
use alloc::sync::Arc;

use alloy_consensus::{
    proofs::calculate_receipt_root, BlockHeader as _, TxReceipt, EMPTY_OMMER_ROOT_HASH,
};
use alloy_primitives::{b64, Address, Bloom, B256, B64, U256};
use core::fmt::Debug;
use reth_chainspec::{EthChainSpec, EthereumHardforks};
use reth_consensus::{Consensus, ConsensusError, FullConsensus, HeaderValidator, ReceiptRootBloom};
use reth_consensus_common::validation::{
    validate_against_parent_hash_number, validate_body_against_header, validate_header_gas,
};
use reth_execution_types::BlockExecutionResult;
use reth_primitives_traits::{
    constants::{GAS_LIMIT_BOUND_DIVISOR, MINIMUM_GAS_LIMIT},
    receipt::gas_spent_by_transactions,
    Block, BlockBody, BlockHeader, GotExpected, NodePrimitives, RecoveredBlock, SealedBlock,
    SealedHeader, SignedTransaction,
};
use reth_scroll_primitives::ScrollReceipt;
use scroll_alloy_consensus::ScrollTransaction;
use scroll_alloy_hardforks::ScrollHardforks;

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
        receipt_root_bloom: Option<ReceiptRootBloom>,
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
            let (receipts_root, logs_bloom) = if let Some((root, bloom)) = receipt_root_bloom {
                (root, bloom)
            } else {
                let receipts_with_bloom = result
                    .receipts
                    .iter()
                    .map(TxReceipt::with_bloom_ref)
                    .collect::<alloc::vec::Vec<_>>();
                let root = calculate_receipt_root(&receipts_with_bloom);
                let bloom = receipts_with_bloom.iter().fold(Bloom::ZERO, |b, r| b | r.bloom_ref());
                (root, bloom)
            };

            if receipts_root != block.header().receipts_root() {
                return Err(ConsensusError::BodyReceiptRootDiff(
                    GotExpected { got: receipts_root, expected: block.header().receipts_root() }
                        .into(),
                ));
            }
            if logs_bloom != block.header().logs_bloom() {
                return Err(ConsensusError::BodyBloomLogDiff(
                    GotExpected { got: logs_bloom, expected: block.header().logs_bloom() }.into(),
                ));
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
        let ts = block.header().timestamp();
        validate_l1_messages(
            block.body().transactions(),
            self.chain_spec.is_euclid_v2_active_at_timestamp(ts),
        )?;

        Ok(())
    }
}

impl<ChainSpec: EthChainSpec + ScrollHardforks, H: BlockHeader> HeaderValidator<H>
    for ScrollBeaconConsensus<ChainSpec>
{
    fn validate_header(&self, header: &SealedHeader<H>) -> Result<(), ConsensusError> {
        validate_header_timestamp(header.header())?;
        validate_header_fields(header.header(), &self.chain_spec)?;
        validate_header_gas(header.header())?;
        validate_header_base_fee(header.header(), &self.chain_spec)?;
        Ok(())
    }

    fn validate_header_against_parent(
        &self,
        header: &SealedHeader<H>,
        parent: &SealedHeader<H>,
    ) -> Result<(), ConsensusError> {
        validate_against_parent_hash_number(header.header(), parent)?;
        validate_against_parent_timestamp(header.header(), parent.header())?;
        validate_against_parent_gas_limit(header.header(), parent.header())?;

        // ensure that the blob gas fields for this block
        if self.chain_spec.blob_params_at_timestamp(header.timestamp()).is_some() {
            return Err(ConsensusError::Other(
                ScrollConsensusError::UnexpectedBlobParams.to_string(),
            ))
        }

        Ok(())
    }
}

#[inline]
fn validate_header_fields<H: BlockHeader, ChainSpec: EthChainSpec + ScrollHardforks>(
    header: &H,
    chain_spec: ChainSpec,
) -> Result<(), ScrollConsensusError> {
    // Common checks to pre and post Euclid v2.
    if header.ommers_hash() != EMPTY_OMMER_ROOT_HASH {
        return Err(ConsensusError::TheMergeOmmerRootIsNotEmpty.into())
    }
    if header.mix_hash() != Some(B256::ZERO) {
        return Err(ScrollConsensusError::MixHashNotZero(header.mix_hash()))
    }

    if chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
        verify_header_fields_post_euclid_v2(header)?;
    } else {
        let clique_config =
            chain_spec.genesis().config.clique.expect("clique config required pre euclid v2");
        let epoch = clique_config.epoch.expect("epoch required pre euclid v2");
        verify_header_fields_pre_euclid_v2(header, epoch)?;
    }

    Ok(())
}

/// Verify the header's field for post Euclid v2 blocks.
#[inline]
fn verify_header_fields_post_euclid_v2<H: BlockHeader>(
    header: &H,
) -> Result<(), ScrollConsensusError> {
    if header.beneficiary() != Address::ZERO {
        return Err(ScrollConsensusError::CoinbaseNotZero(header.beneficiary()))
    }
    if header.nonce() != Some(B64::ZERO) {
        return Err(ScrollConsensusError::NonceNotZero(header.nonce()))
    }
    if header.difficulty() != U256::ONE {
        return Err(ScrollConsensusError::DifficultyNotOne(header.difficulty()))
    }
    if !header.extra_data().is_empty() {
        return Err(ConsensusError::ExtraDataExceedsMax { len: header.extra_data().len() }.into())
    }

    Ok(())
}

/// Verify the header's field for pre Euclid v2 blocks.
#[inline]
fn verify_header_fields_pre_euclid_v2<H: BlockHeader>(
    header: &H,
    epoch: u64,
) -> Result<(), ScrollConsensusError> {
    let is_checkpoint = header.number().is_multiple_of(epoch);
    if is_checkpoint && header.beneficiary() != Address::ZERO {
        return Err(ScrollConsensusError::CoinbaseNotZero(header.beneficiary()))
    }
    if header.nonce() != Some(B64::ZERO) && header.nonce() != Some(b64!("ffffffffffffffff")) {
        return Err(ScrollConsensusError::InvalidCliqueNonce(header.nonce()))
    }
    if is_checkpoint && header.nonce() != Some(B64::ZERO) {
        return Err(ScrollConsensusError::NonceNotZero(header.nonce()))
    }
    if header.extra_data().len() < 32 {
        return Err(ScrollConsensusError::MissingVanity)
    }
    if header.extra_data().len() < 32 + 65 {
        return Err(ScrollConsensusError::MissingSignature)
    }
    let signer_bytes = header.extra_data().len() - 32 - 65;
    if !is_checkpoint && signer_bytes > 0 {
        return Err(ScrollConsensusError::InvalidCheckpointSigners)
    }
    let difficulty = header.difficulty();
    if difficulty != CLIQUE_IN_TURN_DIFFICULTY && difficulty != CLIQUE_NO_TURN_DIFFICULTY {
        return Err(ScrollConsensusError::InvalidCliqueDifficulty(difficulty))
    }

    Ok(())
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
) -> Result<(), ScrollConsensusError> {
    if chain_spec.is_curie_active_at_block(header.number()) {
        if header.base_fee_per_gas().is_none() {
            return Err(ConsensusError::BaseFeeMissing.into())
        }
        // note: we do not verify L2 base fee, the sequencer has the
        // right to set any base fee below the maximum. L2 base fee
        // is not subject to L2 consensus or zk verification.
        if header.base_fee_per_gas().expect("checked") > SCROLL_MAXIMUM_BASE_FEE {
            return Err(ScrollConsensusError::BaseFeeOverLimit)
        }
    }
    if !chain_spec.is_curie_active_at_block(header.number()) && header.base_fee_per_gas().is_some()
    {
        return Err(ScrollConsensusError::UnexpectedBaseFee)
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

/// Validates the gas limit of the block against the parent.
#[inline]
fn validate_against_parent_gas_limit<H: BlockHeader>(
    header: &H,
    parent: &H,
) -> Result<(), ConsensusError> {
    let diff = header.gas_limit().abs_diff(parent.gas_limit());
    let limit = parent.gas_limit() / GAS_LIMIT_BOUND_DIVISOR;
    if diff > limit {
        return if header.gas_limit() > parent.gas_limit() {
            Err(ConsensusError::GasLimitInvalidIncrease {
                parent_gas_limit: parent.gas_limit(),
                child_gas_limit: header.gas_limit(),
            })
        } else {
            Err(ConsensusError::GasLimitInvalidDecrease {
                parent_gas_limit: parent.gas_limit(),
                child_gas_limit: header.gas_limit(),
            })
        }
    }
    // Check that the gas limit is above the minimum allowed gas limit.
    if header.gas_limit() < MINIMUM_GAS_LIMIT {
        return Err(ConsensusError::GasLimitInvalidMinimum { child_gas_limit: header.gas_limit() })
    }
    Ok(())
}

/// Validate the L1 messages by checking they are only present that the start of the block and only
/// have increasing queue index.
#[inline]
fn validate_l1_messages<Tx: SignedTransaction + ScrollTransaction>(
    txs: &[Tx],
    is_euclid_v2: bool,
) -> Result<(), ScrollConsensusError> {
    // Check L1 messages are only at the start of the block and correctly ordered.
    let mut saw_l2_transaction = false;
    let mut queue_index = txs
        .iter()
        .find(|tx| tx.is_l1_message())
        .and_then(|tx| tx.queue_index())
        .unwrap_or_default();

    // starting at EuclidV2, we don't skip L1 messages.
    let l1_message_index_check: fn(u64, u64) -> bool = if is_euclid_v2 {
        |tx_queue_index, queue_index| tx_queue_index != queue_index
    } else {
        |tx_queue_index, queue_index| tx_queue_index < queue_index
    };

    for tx in txs {
        // Check index is strictly increasing pre EuclidV2 and sequential post EuclidV2.
        if tx.is_l1_message() {
            let tx_queue_index = tx.queue_index().expect("is_l1_message");
            if l1_message_index_check(tx_queue_index, queue_index) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScrollConsensusError;

    use alloy_consensus::{Header, Signed, TxEip1559};
    use alloy_primitives::{b64, Address, Bloom, Bytes, Signature, B256, U256};
    use reth_consensus::ConsensusError;
    use reth_primitives_traits::constants::{GAS_LIMIT_BOUND_DIVISOR, MINIMUM_GAS_LIMIT};
    use reth_scroll_chainspec::SCROLL_MAINNET;
    use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};

    fn create_test_header() -> Header {
        Header {
            parent_hash: B256::random(),
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: Address::ZERO,
            state_root: B256::random(),
            transactions_root: B256::random(),
            receipts_root: B256::random(),
            logs_bloom: Bloom::default(),
            difficulty: U256::ONE,
            number: 1,
            gas_limit: 30000000,
            gas_used: 0,
            timestamp: 1000,
            extra_data: Bytes::new(),
            mix_hash: B256::ZERO,
            nonce: B64::ZERO,
            base_fee_per_gas: None,
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
        }
    }

    #[test]
    fn test_validate_header_timestamp_success() {
        let header = create_test_header();
        assert!(validate_header_timestamp(&header).is_ok());
    }

    #[test]
    fn test_validate_header_timestamp_future() {
        let mut header = create_test_header();
        // timestamp in the future.
        header.timestamp = u64::MAX;

        let result = validate_header_timestamp(&header);
        assert!(matches!(result, Err(ConsensusError::TimestampIsInPast { .. })));
    }

    #[test]
    fn test_verify_header_fields_post_euclid_v2_success() {
        let header = create_test_header();
        assert!(verify_header_fields_post_euclid_v2(&header).is_ok());
    }

    #[test]
    fn test_verify_header_fields_post_euclid_v2_coinbase_not_zero() {
        let mut header = create_test_header();
        header.beneficiary = Address::random();

        let result = verify_header_fields_post_euclid_v2(&header);
        assert!(matches!(result, Err(ScrollConsensusError::CoinbaseNotZero(_))));
    }

    #[test]
    fn test_verify_header_fields_post_euclid_v2_nonce_not_zero() {
        let mut header = create_test_header();
        header.nonce = b64!("0123456789abcdef");

        let result = verify_header_fields_post_euclid_v2(&header);
        assert!(matches!(result, Err(ScrollConsensusError::NonceNotZero(_))));
    }

    #[test]
    fn test_verify_header_fields_post_euclid_v2_difficulty_not_one() {
        let mut header = create_test_header();
        header.difficulty = U256::from(2);

        let result = verify_header_fields_post_euclid_v2(&header);
        assert!(matches!(result, Err(ScrollConsensusError::DifficultyNotOne(_))));
    }

    #[test]
    fn test_verify_header_fields_post_euclid_v2_extra_data_not_empty() {
        let mut header = create_test_header();
        header.extra_data = Bytes::from(vec![1, 2, 3]);

        let result = verify_header_fields_post_euclid_v2(&header);
        assert!(matches!(
            result,
            Err(ScrollConsensusError::Eth(ConsensusError::ExtraDataExceedsMax { .. }))
        ));
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_success() {
        let mut header = create_test_header();
        // valid extra data (32 bytes vanity + 65 bytes signature).
        let mut extra_data = vec![0u8; 32];
        extra_data.extend_from_slice(&[0u8; 65]);
        header.extra_data = Bytes::from(extra_data);

        assert!(verify_header_fields_pre_euclid_v2(&header, 30000).is_ok());
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_checkpoint_coinbase_not_zero() {
        let mut header = create_test_header();
        // checkpoint block.
        header.number = 0;
        header.beneficiary = Address::random();
        let mut extra_data = vec![0u8; 32];
        extra_data.extend_from_slice(&[0u8; 65]);
        header.extra_data = Bytes::from(extra_data);

        let result = verify_header_fields_pre_euclid_v2(&header, 30000);
        assert!(matches!(result, Err(ScrollConsensusError::CoinbaseNotZero(_))));
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_invalid_nonce() {
        let mut header = create_test_header();
        // invalid nonce.
        header.nonce = b64!("1234567890abcdef");
        let mut extra_data = vec![0u8; 32];
        extra_data.extend_from_slice(&[0u8; 65]);
        header.extra_data = Bytes::from(extra_data);

        let result = verify_header_fields_pre_euclid_v2(&header, 30000);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidCliqueNonce(_))));
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_missing_vanity() {
        let mut header = create_test_header();
        // vanity too short.
        header.extra_data = Bytes::from(vec![0u8; 31]);

        let result = verify_header_fields_pre_euclid_v2(&header, 30000);
        assert!(matches!(result, Err(ScrollConsensusError::MissingVanity)));
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_missing_signature() {
        let mut header = create_test_header();
        // signature too short.
        header.extra_data = Bytes::from(vec![0u8; 32]);

        let result = verify_header_fields_pre_euclid_v2(&header, 30000);
        assert!(matches!(result, Err(ScrollConsensusError::MissingSignature)));
    }

    #[test]
    fn test_verify_header_fields_pre_euclid_v2_invalid_difficulty() {
        let mut header = create_test_header();
        // invalid difficulty.
        header.difficulty = U256::from(3);
        let mut extra_data = vec![0u8; 32];
        extra_data.extend_from_slice(&[0u8; 65]);
        header.extra_data = Bytes::from(extra_data);

        let result = verify_header_fields_pre_euclid_v2(&header, 30000);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidCliqueDifficulty(_))));
    }

    #[test]
    fn test_validate_against_parent_timestamp_success() {
        let parent = create_test_header();
        let mut header = create_test_header();
        header.timestamp = parent.timestamp + 1;

        assert!(validate_against_parent_timestamp(&header, &parent).is_ok());
    }

    #[test]
    fn test_validate_against_parent_timestamp_same_time() {
        let parent = create_test_header();
        let header = create_test_header();

        assert!(validate_against_parent_timestamp(&header, &parent).is_ok());
    }

    #[test]
    fn test_validate_against_parent_timestamp_in_past() {
        let parent = create_test_header();
        let mut header = create_test_header();
        header.timestamp = parent.timestamp - 1;

        let result = validate_against_parent_timestamp(&header, &parent);
        assert!(matches!(result, Err(ConsensusError::TimestampIsInPast { .. })));
    }

    #[test]
    fn test_validate_against_parent_gas_limit_success() {
        let parent = create_test_header();
        let mut header = create_test_header();
        // small gas increase.
        header.gas_limit = parent.gas_limit + 100;

        assert!(validate_against_parent_gas_limit(&header, &parent).is_ok());
    }

    #[test]
    fn test_validate_against_parent_gas_limit_too_high_increase() {
        let parent = create_test_header();
        let mut header = create_test_header();
        header.gas_limit = parent.gas_limit + parent.gas_limit / GAS_LIMIT_BOUND_DIVISOR + 1;

        let result = validate_against_parent_gas_limit(&header, &parent);
        assert!(matches!(result, Err(ConsensusError::GasLimitInvalidIncrease { .. })));
    }

    #[test]
    fn test_validate_against_parent_gas_limit_too_high_decrease() {
        let parent = create_test_header();
        let mut header = create_test_header();
        header.gas_limit = parent.gas_limit - parent.gas_limit / GAS_LIMIT_BOUND_DIVISOR - 1;

        let result = validate_against_parent_gas_limit(&header, &parent);
        assert!(matches!(result, Err(ConsensusError::GasLimitInvalidDecrease { .. })));
    }

    #[test]
    fn test_validate_against_parent_gas_limit_below_minimum() {
        let mut parent = create_test_header();
        let mut header = create_test_header();
        parent.gas_limit = MINIMUM_GAS_LIMIT + 1;
        header.gas_limit = MINIMUM_GAS_LIMIT - 1;

        let result = validate_against_parent_gas_limit(&header, &parent);
        dbg!(&result);
        assert!(matches!(result, Err(ConsensusError::GasLimitInvalidMinimum { .. })));
    }

    #[test]
    fn test_validate_l1_messages_success() {
        let txs: Vec<ScrollTxEnvelope> = vec![
            TxL1Message { queue_index: 0, ..Default::default() }.into(),
            TxL1Message { queue_index: 1, ..Default::default() }.into(),
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
        ];

        assert!(validate_l1_messages(&txs, true).is_ok());
        assert!(validate_l1_messages(&txs, false).is_ok());
    }

    #[test]
    fn test_validate_l1_messages_empty() {
        let txs: Vec<ScrollTxEnvelope> = vec![];
        assert!(validate_l1_messages(&txs, true).is_ok());
        assert!(validate_l1_messages(&txs, false).is_ok());
    }

    #[test]
    fn test_validate_l1_messages_only_l2() {
        let txs: Vec<ScrollTxEnvelope> = vec![
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
        ];

        assert!(validate_l1_messages(&txs, true).is_ok());
        assert!(validate_l1_messages(&txs, false).is_ok());
    }

    #[test]
    fn test_validate_l1_messages_invalid_order() {
        let txs: Vec<ScrollTxEnvelope> = vec![
            Signed::new_unchecked(
                TxEip1559::default(),
                Signature::new(U256::ZERO, U256::ZERO, false),
                B256::random(),
            )
            .into(),
            TxL1Message { queue_index: 0, ..Default::default() }.into(),
        ];

        let result = validate_l1_messages(&txs, true);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidL1MessageOrder)));
        let result = validate_l1_messages(&txs, false);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidL1MessageOrder)));
    }

    #[test]
    fn test_validate_l1_messages_non_sequential_queue_index() {
        let txs: Vec<ScrollTxEnvelope> = vec![
            TxL1Message { queue_index: 0, ..Default::default() }.into(),
            TxL1Message { queue_index: 2, ..Default::default() }.into(),
        ];

        // ok as it's not decreasing.
        assert!(validate_l1_messages(&txs, false).is_ok());
        // not ok as it's not sequential.
        let result = validate_l1_messages(&txs, true);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidL1MessageOrder)));
    }

    #[test]
    fn test_validate_l1_messages_decreasing_queue_index() {
        let txs: Vec<ScrollTxEnvelope> = vec![
            TxL1Message { queue_index: 1, ..Default::default() }.into(),
            TxL1Message { queue_index: 0, ..Default::default() }.into(),
        ];

        let result = validate_l1_messages(&txs, true);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidL1MessageOrder)));
        let result = validate_l1_messages(&txs, false);
        assert!(matches!(result, Err(ScrollConsensusError::InvalidL1MessageOrder)));
    }

    #[test]
    fn test_validate_header_base_fee_before_curie() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // pre Curie.
        header.number = 500;
        header.base_fee_per_gas = Some(1000000000);

        let result = validate_header_base_fee(&header, &chain_spec);
        assert!(matches!(result, Err(ScrollConsensusError::UnexpectedBaseFee)));
    }

    #[test]
    fn test_validate_header_base_fee_after_curie_missing() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // post Curie.
        header.number = 7096837;
        header.base_fee_per_gas = None;

        let result = validate_header_base_fee(&header, &chain_spec);
        assert!(matches!(result, Err(ScrollConsensusError::Eth(ConsensusError::BaseFeeMissing))));
    }

    #[test]
    fn test_validate_header_base_fee_after_curie_over_limit() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // post Curie.
        header.number = 7096837;
        header.base_fee_per_gas = Some(SCROLL_MAXIMUM_BASE_FEE + 1);

        let result = validate_header_base_fee(&header, &chain_spec);
        assert!(matches!(result, Err(ScrollConsensusError::BaseFeeOverLimit)));
    }

    #[test]
    fn test_validate_header_base_fee_after_curie_valid() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // post Curie.
        header.number = 7096837;
        header.base_fee_per_gas = Some(1000000000);

        let result = validate_header_base_fee(&header, &chain_spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_header_fields_pre_euclid_v2() {
        let mut header = create_test_header();
        // pre Euclid v2.
        header.timestamp = 1745305199;
        // valid extra data for pre-euclid v2.
        let mut extra_data = vec![0u8; 32];
        extra_data.extend_from_slice(&[0u8; 65]);
        header.extra_data = Bytes::from(extra_data);

        assert!(verify_header_fields_pre_euclid_v2(&header, 30000).is_ok());
    }

    #[test]
    fn test_validate_header_fields_post_euclid_v2() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // post Euclid v2.
        header.timestamp = 1745305201;

        assert!(validate_header_fields(&header, &chain_spec).is_ok());
    }

    #[test]
    fn test_validate_header_fields_mix_hash_not_zero() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // invalid mix hash.
        header.mix_hash = B256::random();

        let result = validate_header_fields(&header, &chain_spec);
        assert!(matches!(result, Err(ScrollConsensusError::MixHashNotZero(_))));
    }

    #[test]
    fn test_validate_header_fields_ommers_hash_not_empty() {
        let chain_spec = SCROLL_MAINNET.clone();

        let mut header = create_test_header();
        // invalid ommer hash.
        header.ommers_hash = B256::random();

        let result = validate_header_fields(&header, &chain_spec);
        assert!(matches!(
            result,
            Err(ScrollConsensusError::Eth(ConsensusError::TheMergeOmmerRootIsNotEmpty))
        ));
    }
}
