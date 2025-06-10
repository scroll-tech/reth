//! Configuration for the payload builder.

use std::fmt::Debug;

/// An instance of the trait can be called to break out of an execution loop.
pub trait Breaker: Debug + Clone + Send + Sync {
    /// Return whether the loop should be broken.
    fn should_break(&self) -> bool;
}

/// Settings for the Scroll builder.
#[derive(Clone, Debug)]
pub struct ScrollBuilderConfig<B> {
    /// Desired gas limit.
    pub desired_gas_limit: u64,
    /// Returns true if execution loop should be exited.
    pub breaker: B,
}

impl<B> ScrollBuilderConfig<B> {
    /// Returns a new instance of [`ScrollBuilderConfig`].
    pub const fn new(desired_gas_limit: u64, breaker: B) -> Self {
        Self { desired_gas_limit, breaker }
    }
}
