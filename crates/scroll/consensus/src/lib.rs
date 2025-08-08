//! Scroll consensus implementation.

extern crate alloc;

mod constants;
pub use constants::MAX_ROLLUP_FEE;

mod error;
pub use error::ScrollConsensusError;

mod validation;
pub use validation::ScrollBeaconConsensus;
