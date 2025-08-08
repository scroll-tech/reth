//! Scroll consensus implementation.

extern crate alloc;

mod constants;
pub use constants::{CLIQUE_IN_TURN_DIFFICULTY, CLIQUE_NO_TURN_DIFFICULTY};

mod error;
pub use error::ScrollConsensusError;

mod validation;
pub use validation::ScrollBeaconConsensus;
