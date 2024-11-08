//! Scroll `revm` primitives types redefinitions.

use revm::{
    precompile::HashMap,
    primitives::{b256, AccountInfo, Bytecode, B256, KECCAK_EMPTY, U256},
};

/// A Keccak code hash.
type KeccakHash = B256;
/// A Poseidon code hash.
type PoseidonHash = B256;
/// Size of a contract's code in bytes.
type CodeSize = usize;

/// Scroll post execution context maps a Keccak code hash of a contract's bytecode to its code size
/// and Poseidon code hash.
pub type ScrollPostExecutionContext = HashMap<KeccakHash, (CodeSize, PoseidonHash)>;

/// The Poseidon hash of the empty string `""`.
pub const POSEIDON_EMPTY: B256 =
    b256!("2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864");

/// Poseidon code hash
pub fn poseidon(code: &[u8]) -> B256 {
    poseidon_bn254::hash_code(code).into()
}

/// The Scroll account information. Code copy of [`AccountInfo`]. Provides additional `code_size`
/// and `poseidon_code_hash` fields needed in the state root computation.
#[derive(Clone, Debug, Eq)]
pub struct ScrollAccountInfo {
    /// Account balance.
    pub balance: U256,
    /// Account nonce.
    pub nonce: u64,
    /// Account code keccak hash.
    pub code_hash: B256,
    /// code: if None, `code_by_hash` will be used to fetch it if code needs to be loaded from
    /// inside `revm`.
    pub code: Option<Bytecode>,
    /// Account code size.
    pub code_size: usize,
    /// Account code Poseidon hash. [`POSEIDON_EMPTY`] if code is None or empty.
    pub poseidon_code_hash: B256,
}

impl From<(AccountInfo, &ScrollPostExecutionContext)> for ScrollAccountInfo {
    fn from((info, context): (AccountInfo, &ScrollPostExecutionContext)) -> Self {
        let (code_size, poseidon_code_hash) =
            context.get(&info.code_hash).copied().unwrap_or((0, POSEIDON_EMPTY));
        Self {
            balance: info.balance,
            nonce: info.nonce,
            code_hash: info.code_hash,
            code: info.code,
            code_size,
            poseidon_code_hash,
        }
    }
}

impl Default for ScrollAccountInfo {
    fn default() -> Self {
        Self {
            balance: U256::ZERO,
            code_hash: KECCAK_EMPTY,
            code: Some(Bytecode::default()),
            nonce: 0,
            code_size: 0,
            poseidon_code_hash: POSEIDON_EMPTY,
        }
    }
}

impl PartialEq for ScrollAccountInfo {
    fn eq(&self, other: &Self) -> bool {
        self.balance == other.balance &&
            self.nonce == other.nonce &&
            self.code_hash == other.code_hash
    }
}

impl ScrollAccountInfo {
    /// Creates a new [`ScrollAccountInfo`] with the given fields.
    pub fn new(
        balance: U256,
        nonce: u64,
        code_hash: B256,
        code: Bytecode,
        poseidon_code_hash: B256,
    ) -> Self {
        let code_size = code.len();
        Self { balance, nonce, code: Some(code), code_hash, code_size, poseidon_code_hash }
    }

    /// Returns account info without the code.
    pub fn without_code(mut self) -> Self {
        self.take_bytecode();
        self
    }

    /// Returns if an account is empty.
    ///
    /// An account is empty if the following conditions are met.
    /// - code hash is zero or set to the Keccak256 hash of the empty string `""`
    /// - balance is zero
    /// - nonce is zero
    pub fn is_empty(&self) -> bool {
        let code_empty = self.is_empty_code_hash() || self.code_hash.is_zero();
        code_empty && self.balance.is_zero() && self.nonce == 0
    }

    /// Returns `true` if the account is not empty.
    pub fn exists(&self) -> bool {
        !self.is_empty()
    }

    /// Returns `true` if account has no nonce and code.
    pub fn has_no_code_and_nonce(&self) -> bool {
        self.is_empty_code_hash() && self.nonce == 0
    }

    /// Return bytecode hash associated with this account.
    /// If account does not have code, it returns `KECCAK_EMPTY` hash.
    pub const fn code_hash(&self) -> B256 {
        self.code_hash
    }

    /// Returns true if the code hash is the Keccak256 hash of the empty string `""`.
    #[inline]
    pub fn is_empty_code_hash(&self) -> bool {
        self.code_hash == KECCAK_EMPTY
    }

    /// Take bytecode from account. Code will be set to None.
    pub fn take_bytecode(&mut self) -> Option<Bytecode> {
        self.code.take()
    }

    /// Returns a [`ScrollAccountInfo`] with only balance.
    pub fn from_balance(balance: U256) -> Self {
        Self { balance, ..Default::default() }
    }

    /// Returns a [`ScrollAccountInfo`] with defaults for balance and nonce.
    /// Computes the Keccak and Poseidon hash of the provided bytecode.
    pub fn from_bytecode(bytecode: Bytecode) -> Self {
        let hash = bytecode.hash_slow();
        let code_size = bytecode.len();
        let poseidon_code_hash = poseidon(bytecode.bytecode());

        Self {
            balance: U256::ZERO,
            nonce: 1,
            code: Some(bytecode),
            code_hash: hash,
            code_size,
            poseidon_code_hash,
        }
    }
}
