//! Primitive types for the Scroll extension of `Reth`.

pub use execution_context::ScrollPostExecutionContext;
mod execution_context;

pub use poseidon::{poseidon, POSEIDON_EMPTY};
mod poseidon;
