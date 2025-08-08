use alloy_primitives::U256;

/// The maximum value Rollup fee.
pub const MAX_ROLLUP_FEE: U256 = U256::from_limbs([u64::MAX, 0, 0, 0]);
