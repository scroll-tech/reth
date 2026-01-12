//! Configuration for the payload builder.

use core::time::Duration;
use reth_chainspec::MIN_TRANSACTION_GAS;
use reth_primitives_traits::constants::GAS_LIMIT_BOUND_DIVISOR;
use std::{fmt::Debug, time::Instant};

/// Settings for the Scroll builder.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ScrollBuilderConfig {
    /// Gas limit.
    pub gas_limit: Option<u64>,
    /// Time limit for payload building.
    pub time_limit: Duration,
    /// Maximum total data availability size for a block.
    pub max_da_block_size: Option<u64>,
}

/// Minimal data bytes size per transaction.
pub const MIN_TRANSACTION_DATA_SIZE: u64 = 115u64;

impl ScrollBuilderConfig {
    /// Returns a new instance of [`ScrollBuilderConfig`].
    pub const fn new(
        gas_limit: Option<u64>,
        time_limit: Duration,
        max_da_block_size: Option<u64>,
    ) -> Self {
        Self { gas_limit, time_limit, max_da_block_size }
    }

    /// Returns the [`PayloadBuildingBreaker`] for the config with the actual gas limit used.
    ///
    /// The `actual_gas_limit` should be the gas limit after clamping based on parent's gas limit.
    pub(super) fn breaker_with_gas_limit(&self, actual_gas_limit: u64) -> PayloadBuildingBreaker {
        PayloadBuildingBreaker::new(self.time_limit, Some(actual_gas_limit), self.max_da_block_size)
    }
}

/// Used in the [`super::ScrollPayloadBuilder`] to exit the transactions execution loop early.
#[derive(Debug, Clone)]
pub struct PayloadBuildingBreaker {
    start: Instant,
    time_limit: Duration,
    gas_limit: Option<u64>,
    max_da_block_size: Option<u64>,
}

impl PayloadBuildingBreaker {
    /// Returns a new instance of the [`PayloadBuildingBreaker`].
    fn new(time_limit: Duration, gas_limit: Option<u64>, max_da_block_size: Option<u64>) -> Self {
        Self { start: Instant::now(), time_limit, gas_limit, max_da_block_size }
    }

    /// Returns whether the payload building should stop.
    pub(super) fn should_break(
        &self,
        cumulative_gas_used: u64,
        cumulative_da_size_used: u64,
    ) -> bool {
        // Check time limit
        if self.start.elapsed() >= self.time_limit {
            return true;
        }

        // Check gas limit if configured
        if let Some(gas_limit) = self.gas_limit &&
            cumulative_gas_used > gas_limit.saturating_sub(MIN_TRANSACTION_GAS)
        {
            return true;
        }

        // Check data availability size limit if configured
        if let Some(max_size) = self.max_da_block_size &&
            cumulative_da_size_used > max_size.saturating_sub(MIN_TRANSACTION_DATA_SIZE)
        {
            return true;
        }

        false
    }
}

/// Calculate the gas limit for the next block based on parent and desired gas limits.
///
/// The gas limit can only change by at most `parent_gas_limit / 1024` per block.
/// Ref: <https://github.com/ethereum/go-ethereum/blob/88cbfab332c96edfbe99d161d9df6a40721bd786/core/block_validator.go#L166>
pub fn calculate_block_gas_limit(parent_gas_limit: u64, desired_gas_limit: u64) -> u64 {
    let delta = (parent_gas_limit / GAS_LIMIT_BOUND_DIVISOR).saturating_sub(1);
    let min_gas_limit = parent_gas_limit.saturating_sub(delta);
    let max_gas_limit = parent_gas_limit.saturating_add(delta);
    desired_gas_limit.clamp(min_gas_limit, max_gas_limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_break_on_time_limit() {
        let breaker = PayloadBuildingBreaker::new(
            Duration::from_millis(200),
            Some(2 * MIN_TRANSACTION_GAS),
            Some(2 * MIN_TRANSACTION_DATA_SIZE),
        );
        assert!(!breaker.should_break(MIN_TRANSACTION_GAS, MIN_TRANSACTION_DATA_SIZE));
        std::thread::sleep(Duration::from_millis(201));
        assert!(breaker.should_break(MIN_TRANSACTION_GAS, MIN_TRANSACTION_DATA_SIZE));
    }

    #[test]
    fn test_should_break_on_gas_limit() {
        let breaker = PayloadBuildingBreaker::new(
            Duration::from_secs(1),
            Some(2 * MIN_TRANSACTION_GAS),
            Some(2 * MIN_TRANSACTION_DATA_SIZE),
        );
        assert!(!breaker.should_break(MIN_TRANSACTION_GAS, MIN_TRANSACTION_DATA_SIZE));
        assert!(breaker.should_break(MIN_TRANSACTION_GAS + 1, MIN_TRANSACTION_DATA_SIZE));
    }

    #[test]
    fn test_should_break_on_data_size_limit() {
        let breaker = PayloadBuildingBreaker::new(
            Duration::from_secs(1),
            Some(2 * MIN_TRANSACTION_GAS),
            Some(2 * MIN_TRANSACTION_DATA_SIZE),
        );
        assert!(!breaker.should_break(MIN_TRANSACTION_GAS, MIN_TRANSACTION_DATA_SIZE));
        assert!(breaker.should_break(MIN_TRANSACTION_GAS, MIN_TRANSACTION_DATA_SIZE + 1));
    }

    #[test]
    fn test_should_break_with_no_da_limit() {
        let breaker = PayloadBuildingBreaker::new(
            Duration::from_secs(1),
            Some(2 * MIN_TRANSACTION_GAS),
            None, // No DA limit
        );
        // Should not break on large DA size when no limit is set
        assert!(!breaker.should_break(MIN_TRANSACTION_GAS, u64::MAX));
        // But should still break on gas limit
        assert!(breaker.should_break(MIN_TRANSACTION_GAS + 1, u64::MAX));
    }

    #[test]
    fn test_calculate_block_gas_limit_within_bounds() {
        let parent_gas_limit = GAS_LIMIT_BOUND_DIVISOR * 10; // 10240
        let delta = parent_gas_limit / GAS_LIMIT_BOUND_DIVISOR - 1; // 9

        // Desired equals parent - should return parent
        assert_eq!(calculate_block_gas_limit(parent_gas_limit, parent_gas_limit), parent_gas_limit);

        // Small increase within bounds
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit + 5),
            parent_gas_limit + 5
        );

        // Small decrease within bounds
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit - 5),
            parent_gas_limit - 5
        );

        // Exactly at max allowed increase
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit + delta),
            parent_gas_limit + delta
        );

        // Exactly at max allowed decrease
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit - delta),
            parent_gas_limit - delta
        );
    }

    #[test]
    fn test_calculate_block_gas_limit_clamped_increase() {
        let parent_gas_limit = GAS_LIMIT_BOUND_DIVISOR * 10; // 10240
        let delta = parent_gas_limit / GAS_LIMIT_BOUND_DIVISOR - 1; // 9
        let max_gas_limit = parent_gas_limit + delta;

        // Desired exceeds max - should clamp to max
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit + delta + 1),
            max_gas_limit
        );

        // Large increase - should clamp
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit * 2),
            max_gas_limit
        );
    }

    #[test]
    fn test_calculate_block_gas_limit_clamped_decrease() {
        let parent_gas_limit = GAS_LIMIT_BOUND_DIVISOR * 10; // 10240
        let delta = parent_gas_limit / GAS_LIMIT_BOUND_DIVISOR - 1; // 9
        let min_gas_limit = parent_gas_limit - delta;

        // Desired below min - should clamp to min
        assert_eq!(
            calculate_block_gas_limit(parent_gas_limit, parent_gas_limit - delta - 1),
            min_gas_limit
        );

        // Much lower than allowed - should clamp
        assert_eq!(calculate_block_gas_limit(parent_gas_limit, 0), min_gas_limit);
    }
}
