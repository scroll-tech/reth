//! Engine Payload related types.

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(not(feature = "std"))]
extern crate alloc as std;

pub use base_fee::{
    PayloadBuildingBaseFeeProvider, L1_BASE_FEE_SLOT, L1_BASE_FEE_VERIFICATION_FEE_DIVIDER,
    L1_BASE_FEE_VERIFICATION_FEE_MULTIPLIER, L2_SEQUENCER_FEE, MAX_L2_BASE_FEE, PROVING_FEE,
};
mod base_fee;

pub mod builder;
pub use builder::{ScrollPayloadBuilder, ScrollPayloadTransactions};

mod error;
pub use error::ScrollPayloadBuilderError;

#[cfg(feature = "test-utils")]
mod test_utils;

#[cfg(feature = "test-utils")]
pub use test_utils::{NoopPayloadJob, NoopPayloadJobGenerator};
