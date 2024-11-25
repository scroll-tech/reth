//! Scroll execution related implementations.

#![warn(unused_crate_dependencies)]

#[cfg(any(not(feature = "scroll"), feature = "test-utils"))]
pub use context::DEFAULT_EMPTY_CONTEXT;
pub use context::{ContextFul, ExecutionContext, FinalizeExecution, WithContext};
mod context;
