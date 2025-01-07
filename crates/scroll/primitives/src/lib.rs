//! Primitive types for the Scroll extension of `Reth`.

#![warn(unused_crate_dependencies)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use execution_context::ScrollPostExecutionContext;
mod execution_context;

pub use account_extension::AccountExtension;
mod account_extension;

/// Poseidon hashing primitives.
pub mod poseidon;
