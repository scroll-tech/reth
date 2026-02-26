use crate::addons::ScrollNodeTypes;
use std::sync::Arc;

use alloy_consensus::BlockHeader;
use alloy_rpc_types_engine::{ExecutionData, PayloadError};
use reth_node_api::{
    AddOnsContext, EngineApiMessageVersion, EngineApiValidator, EngineObjectValidationError,
    ExecutionPayload, FullNodeComponents, InvalidPayloadAttributesError, MessageValidationKind,
    NewPayloadError, PayloadAttributes, PayloadOrAttributes, PayloadTypes, PayloadValidator,
    VersionSpecificValidationError,
};
use reth_node_builder::rpc::PayloadValidatorBuilder;
use reth_node_types::NodeTypes;
use reth_primitives_traits::{Block, SealedBlock};
use reth_scroll_consensus::{CLIQUE_IN_TURN_DIFFICULTY, CLIQUE_NO_TURN_DIFFICULTY};
use reth_scroll_engine_primitives::try_into_block;
use reth_scroll_primitives::ScrollBlock;
use scroll_alloy_hardforks::ScrollHardforks;
use scroll_alloy_rpc_types_engine::ScrollPayloadAttributes;

/// Builder for [`ScrollEngineValidator`].
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct ScrollEngineValidatorBuilder;

impl<Node> PayloadValidatorBuilder<Node> for ScrollEngineValidatorBuilder
where
    Node: FullNodeComponents<Types: ScrollNodeTypes>,
{
    type Validator = ScrollEngineValidator<<Node::Types as NodeTypes>::ChainSpec>;

    async fn build(self, ctx: &AddOnsContext<'_, Node>) -> eyre::Result<Self::Validator> {
        Ok(ScrollEngineValidator::new(ctx.config.chain.clone()))
    }
}

/// Scroll engine validator.
#[derive(Debug, Clone)]
pub struct ScrollEngineValidator<CS> {
    chainspec: Arc<CS>,
}

impl<CS> ScrollEngineValidator<CS> {
    /// Returns a new [`ScrollEngineValidator`].
    pub const fn new(chainspec: Arc<CS>) -> Self {
        Self { chainspec }
    }
}

impl<CS, Types> EngineApiValidator<Types> for ScrollEngineValidator<CS>
where
    Types: PayloadTypes<PayloadAttributes = ScrollPayloadAttributes, ExecutionData = ExecutionData>,
    CS: ScrollHardforks + Send + Sync + 'static,
{
    fn validate_version_specific_fields(
        &self,
        _version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, Types::ExecutionData, ScrollPayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_scroll_payload_or_attributes(
            &payload_or_attrs,
            payload_or_attrs.message_validation_kind(),
        )?;
        Ok(())
    }

    fn ensure_well_formed_attributes(
        &self,
        _version: EngineApiMessageVersion,
        attributes: &ScrollPayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_scroll_payload_or_attributes(
            &PayloadOrAttributes::PayloadAttributes::<'_, ExecutionData, _>(attributes),
            MessageValidationKind::PayloadAttributes,
        )?;

        // ensure block data hint is present pre euclid.
        let is_euclid_active =
            self.chainspec.is_euclid_active_at_timestamp(attributes.payload_attributes.timestamp);
        if !is_euclid_active && attributes.block_data_hint.is_empty() {
            return Err(EngineObjectValidationError::InvalidParams(
                "Missing block data hint Pre-Euclid".to_string().into(),
            ));
        }

        Ok(())
    }
}

/// Validates the payload or attributes for Scroll.
fn validate_scroll_payload_or_attributes<Payload: ExecutionPayload>(
    payload_or_attributes: &PayloadOrAttributes<'_, Payload, ScrollPayloadAttributes>,
    message_validation_kind: MessageValidationKind,
) -> Result<(), EngineObjectValidationError> {
    if payload_or_attributes.parent_beacon_block_root().is_some() {
        return Err(message_validation_kind
            .to_error(VersionSpecificValidationError::ParentBeaconBlockRootNotSupportedBeforeV3));
    }
    if payload_or_attributes.withdrawals().is_some() {
        return Err(message_validation_kind
            .to_error(VersionSpecificValidationError::HasWithdrawalsPreShanghai));
    }

    Ok(())
}

impl<CS, Types> PayloadValidator<Types> for ScrollEngineValidator<CS>
where
    Types: PayloadTypes<ExecutionData = ExecutionData>,
    CS: ScrollHardforks + Send + Sync + 'static,
{
    type Block = ScrollBlock;

    fn convert_payload_to_block(
        &self,
        payload: ExecutionData,
    ) -> Result<SealedBlock<Self::Block>, NewPayloadError> {
        let expected_hash = payload.payload.block_hash();

        // First parse the block
        let mut block = try_into_block(payload, self.chainspec.clone())?;

        // Seal the block with the no-turn difficulty and return if hashes match.
        // We guess the difficulty, which should always be 1 or 2 on Scroll.
        // CLIQUE_NO_TURN_DIFFICULTY is used starting at Euclid, so we test this value first.
        block.header.difficulty = CLIQUE_NO_TURN_DIFFICULTY;
        let block_hash_no_turn = block.hash_slow();
        if block_hash_no_turn == expected_hash {
            return Ok(block.seal_unchecked(block_hash_no_turn));
        }

        // Seal the block with the in-turn difficulty and return if hashes match
        block.header.difficulty = CLIQUE_IN_TURN_DIFFICULTY;
        let block_hash_in_turn = block.hash_slow();
        if block_hash_in_turn == expected_hash {
            return Ok(block.seal_unchecked(block_hash_in_turn));
        }

        Err(PayloadError::BlockHash { execution: block_hash_no_turn, consensus: expected_hash }
            .into())
    }

    fn validate_payload_attributes_against_header(
        &self,
        attr: &Types::PayloadAttributes,
        header: &<Self::Block as Block>::Header,
    ) -> Result<(), InvalidPayloadAttributesError> {
        if attr.timestamp() < header.timestamp() {
            return Err(InvalidPayloadAttributesError::InvalidTimestamp);
        }
        Ok(())
    }
}
