//! Scroll consensus implementation.

pub use curie::{
    apply_curie_hard_fork, CURIE_L1_GAS_PRICE_ORACLE_BYTECODE, CURIE_L1_GAS_PRICE_ORACLE_STORAGE,
    L1_GAS_PRICE_ORACLE_ADDRESS,
};
mod curie;
