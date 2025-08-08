use alloy_primitives::U256;

/// The block difficulty for in turn signing in the Clique consensus.
pub const CLIQUE_IN_TURN_DIFFICULTY: U256 = U256::from_limbs([2, 0, 0, 0]);
/// The block difficulty for out of turn signing in the Clique consensus.
pub const CLIQUE_NO_TURN_DIFFICULTY: U256 = U256::from_limbs([1, 0, 0, 0]);
