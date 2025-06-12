//! Configuration for the payload builder.

use core::time::Duration;
use std::{fmt::Debug, sync::Arc};

/// Provides a blanket implementation for a function returning a
/// [`Breaker`] from a [`ScrollBuilderConfig`].
pub trait ScrollBreakerProvider:
    Fn(&ScrollBuilderConfig) -> Arc<dyn Breaker> + Send + Sync
{
}
impl<T> ScrollBreakerProvider for T where
    T: Fn(&ScrollBuilderConfig) -> Arc<dyn Breaker> + Send + Sync
{
}

/// An instance of the trait can be called to break out of an execution loop.
pub trait Breaker: Debug + Send + Sync {
    /// Return whether the loop should be broken.
    fn should_break(&self) -> bool;
}

impl Breaker for () {
    fn should_break(&self) -> bool {
        false
    }
}

/// Settings for the Scroll builder.
#[derive(Debug, Clone)]
pub struct ScrollBuilderConfig {
    /// Desired gas limit.
    pub desired_gas_limit: u64,
    /// Desired duration of the execution in ms.
    pub desired_execution_time_limit: Duration,
}

impl ScrollBuilderConfig {
    /// Returns a new instance of [`ScrollBuilderConfig`].
    pub const fn new(desired_gas_limit: u64, desired_execution_time_limit: Duration) -> Self {
        Self { desired_gas_limit, desired_execution_time_limit }
    }
}
