use alloy_primitives::U256;
use revm::{database::State, Database};
use scroll_alloy_evm::curie::L1_GAS_PRICE_ORACLE_ADDRESS;

/// L1 gas price oracle base fee slot.
pub const L1_BASE_FEE_SLOT: U256 = U256::from_limbs([1, 0, 0, 0]);

/// Protocol-enforced maximum L2 base fee.
pub const MAX_L2_BASE_FEE: U256 = U256::from_limbs([10_000_000_000, 0, 0, 0]);

/// The fee perceived by the sequencer.
pub const L2_SEQUENCER_FEE: U256 = U256::from_limbs([1_000_000, 0, 0, 0]);

/// The fee related to proving.
pub const PROVING_FEE: U256 = U256::from_limbs([14_680_000, 0, 0, 0]);

/// The multiplier applied on the L1 base to compute verification costs.
pub const L1_BASE_FEE_VERIFICATION_FEE_MULTIPLIER: U256 = U256::from_limbs([34, 0, 0, 0]);

/// The divider applied on the L1 base to compute verification costs.
pub const L1_BASE_FEE_VERIFICATION_FEE_DIVIDER: U256 = U256::from_limbs([1_000_000, 0, 0, 0]);

/// An instance of the trait can return the current base fee for block building.
pub trait PayloadBuildingBaseFeeProvider {
    /// The error type.
    type Error;

    /// Returns the base fee for block building.
    fn payload_building_base_fee(&mut self) -> Result<U256, Self::Error>;
}

impl<DB> PayloadBuildingBaseFeeProvider for State<DB>
where
    DB: Database,
{
    type Error = DB::Error;

    fn payload_building_base_fee(&mut self) -> Result<U256, Self::Error> {
        // load account into cache.
        let _ = self.load_cache_account(L1_GAS_PRICE_ORACLE_ADDRESS)?;

        // query storage.
        let parent_l1_base_fee = self.storage(L1_GAS_PRICE_ORACLE_ADDRESS, L1_BASE_FEE_SLOT)?;

        // L1_base_fee * 0.000034
        let verification_fee = parent_l1_base_fee * L1_BASE_FEE_VERIFICATION_FEE_MULTIPLIER /
            L1_BASE_FEE_VERIFICATION_FEE_DIVIDER;

        let mut base_fee = L2_SEQUENCER_FEE + PROVING_FEE + verification_fee;

        if base_fee > MAX_L2_BASE_FEE {
            base_fee = MAX_L2_BASE_FEE;
        }

        Ok(base_fee)
    }
}
