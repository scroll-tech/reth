#![doc = include_str!("../README.md")]

#[macro_use]
#[allow(unused_imports)]
extern crate alloc;

mod branch;
mod hash_builder;
pub use hash_builder::HashBuilder;
mod leaf;
mod sub_tree;

/// The hashing domain for leaf nodes.
pub const LEAF_NODE_DOMAIN: u64 = 4;

/// The hashing domain for a branch node with two terminal children.
pub const BRANCH_NODE_LTRT_DOMAIN: u64 = 6;

/// The hashing domain for a branch node with a left terminal child and a right branch child.
pub const BRANCH_NODE_LTRB_DOMAIN: u64 = 7;

/// The hashing domain for a branch node with a left branch child and a right terminal child.
pub const BRANCH_NODE_LBRT_DOMAIN: u64 = 8;

/// The hashing domain for a branch node with two branch children.
pub const BRANCH_NODE_LBRB_DOMAIN: u64 = 9;
