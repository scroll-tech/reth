use crate::primitives::ScrollAccountInfo;
use revm::{
    db::states::{PlainStorageChangeset, PlainStorageRevert},
    primitives::{Address, Bytecode, B256},
};

/// Code copy equivalent of the [`revm::db::states::changes::StateChangeset`] to accommodate for the
/// [`ScrollAccountInfo`].
#[derive(Debug)]
pub struct ScrollStateChangeset {
    /// Vector of **not** sorted accounts information.
    pub accounts: Vec<(Address, Option<ScrollAccountInfo>)>,
    /// Vector of **not** sorted storage.
    pub storage: Vec<PlainStorageChangeset>,
    /// Vector of contracts by bytecode hash. **not** sorted.
    pub contracts: Vec<(B256, Bytecode)>,
}

/// Code copy of the [`revm::db::states::changes::PlainStateReverts`] to accommodate for
/// [`ScrollAccountInfo`].
#[derive(Clone, Debug, Default)]
pub struct ScrollPlainStateReverts {
    /// Vector of account with removed contracts bytecode
    ///
    /// Note: If [`ScrollAccountInfo`] is None means that account needs to be removed.
    pub accounts: Vec<Vec<(Address, Option<ScrollAccountInfo>)>>,
    /// Vector of storage with its address.
    pub storage: Vec<Vec<PlainStorageRevert>>,
}

impl ScrollPlainStateReverts {
    /// Constructs new [`ScrollPlainStateReverts`] with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self { accounts: Vec::with_capacity(capacity), storage: Vec::with_capacity(capacity) }
    }
}
