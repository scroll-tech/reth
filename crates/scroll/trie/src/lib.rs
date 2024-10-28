#![doc = include_str!("../README.md")]

#[macro_use]
#[allow(unused_imports)]
extern crate alloc;

mod branch;
mod hash_builder;
pub use hash_builder::HashBuilder;
mod extension;
