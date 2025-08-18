use alloy_primitives::U256;

/// The maximum value Rollup fee.
pub const MAX_ROLLUP_FEE: U256 = U256::from_limbs([u64::MAX, 0, 0, 0]);

/// The block difficulty for in turn signing in the Clique consensus.
pub const CLIQUE_IN_TURN_DIFFICULTY: U256 = U256::from_limbs([2, 0, 0, 0]);

/// The block difficulty for out of turn signing in the Clique consensus.
pub const CLIQUE_NO_TURN_DIFFICULTY: U256 = U256::from_limbs([1, 0, 0, 0]);

/// Maximum allowed base fee. We would only go above this if L1 base fee hits 2931 Gwei.
pub const SCROLL_MAXIMUM_BASE_FEE: u64 = 10000000000;
