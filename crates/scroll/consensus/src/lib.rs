//! Scroll consensus implementation.

extern crate alloc;

mod constants;
pub use constants::{
    CLIQUE_IN_TURN_DIFFICULTY, CLIQUE_NO_TURN_DIFFICULTY, MAX_ROLLUP_FEE, SCROLL_MAXIMUM_BASE_FEE,
};

mod error;
pub use error::ScrollConsensusError;

mod validation;
pub use validation::ScrollBeaconConsensus;
