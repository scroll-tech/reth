//! Scroll `revm` types redefinitions. Account types are redefined with two additional fields
//! `code_size` and `poseidon_code_hash`, which are used during computation of the state root.

pub mod states;

pub mod primitives;

pub use primitives::ScrollAccountInfo;
pub use revm::primitives::*;
