use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_genesis::GenesisAccount;
use alloy_primitives::{keccak256, Bytes, B256, U256};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::Buf;
use derive_more::Deref;
use reth_codecs::{add_arbitrary_tests, Compact};
use revm_primitives::{AccountInfo, Bytecode as RevmBytecode, BytecodeDecodeError, JumpTable};
use serde::{Deserialize, Serialize};

/// Identifier for [`LegacyRaw`](RevmBytecode::LegacyRaw).
const LEGACY_RAW_BYTECODE_ID: u8 = 0;

/// Identifier for removed bytecode variant.
const REMOVED_BYTECODE_ID: u8 = 1;

/// Identifier for [`LegacyAnalyzed`](RevmBytecode::LegacyAnalyzed).
const LEGACY_ANALYZED_BYTECODE_ID: u8 = 2;

/// Identifier for [`Eof`](RevmBytecode::Eof).
const EOF_BYTECODE_ID: u8 = 3;

/// Identifier for [`Eip7702`](RevmBytecode::Eip7702).
const EIP7702_BYTECODE_ID: u8 = 4;

/// An Ethereum account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Compact)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[add_arbitrary_tests(compact)]
pub struct Account {
    /// Account nonce.
    pub nonce: u64,
    /// Account balance.
    pub balance: U256,
    /// Hash of the account's bytecode.
    pub bytecode_hash: Option<B256>,
    /// The extension for a Scroll account. This `Option` should always be `Some` and is used
    /// in order to maintain backward compatibility in case additional fields are added on the
    /// `Account` due to the way storage compaction is performed.
    /// Adding the `code_size` and the `poseidon_code_hash` fields on the `Account` without the
    /// extension caused the used bits of the bitflag struct to reach 16 bits, meaning no
    /// additional bitflag was available. See [reth codecs](reth_codecs::test_utils) for more
    /// details.
    #[cfg(feature = "scroll")]
    pub account_extension: Option<reth_scroll_primitives::AccountExtension>,
}

impl Account {
    /// Whether the account has bytecode.
    pub const fn has_bytecode(&self) -> bool {
        self.bytecode_hash.is_some()
    }

    /// After `SpuriousDragon` empty account is defined as account with nonce == 0 && balance == 0
    /// && bytecode = None (or hash is [`KECCAK_EMPTY`]).
    pub fn is_empty(&self) -> bool {
        self.nonce == 0 &&
            self.balance.is_zero() &&
            self.bytecode_hash.map_or(true, |hash| hash == KECCAK_EMPTY)
    }

    /// Returns an account bytecode's hash.
    /// In case of no bytecode, returns [`KECCAK_EMPTY`].
    pub fn get_bytecode_hash(&self) -> B256 {
        self.bytecode_hash.unwrap_or(KECCAK_EMPTY)
    }
}

/// Bytecode for an account.
///
/// A wrapper around [`revm::primitives::Bytecode`][RevmBytecode] with encoding/decoding support.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Deref)]
pub struct Bytecode(pub RevmBytecode);

impl Bytecode {
    /// Create new bytecode from raw bytes.
    ///
    /// No analysis will be performed.
    ///
    /// # Panics
    ///
    /// Panics if bytecode is EOF and has incorrect format.
    pub fn new_raw(bytes: Bytes) -> Self {
        Self(RevmBytecode::new_raw(bytes))
    }

    /// Creates a new raw [`revm_primitives::Bytecode`].
    ///
    /// Returns an error on incorrect Bytecode format.
    #[inline]
    pub fn new_raw_checked(bytecode: Bytes) -> Result<Self, BytecodeDecodeError> {
        RevmBytecode::new_raw_checked(bytecode).map(Self)
    }
}

impl Compact for Bytecode {
    fn to_compact<B>(&self, buf: &mut B) -> usize
    where
        B: bytes::BufMut + AsMut<[u8]>,
    {
        let bytecode = match &self.0 {
            RevmBytecode::LegacyRaw(bytes) => bytes,
            RevmBytecode::LegacyAnalyzed(analyzed) => analyzed.bytecode(),
            RevmBytecode::Eof(eof) => eof.raw(),
            RevmBytecode::Eip7702(eip7702) => eip7702.raw(),
        };
        buf.put_u32(bytecode.len() as u32);
        buf.put_slice(bytecode.as_ref());
        let len = match &self.0 {
            RevmBytecode::LegacyRaw(_) => {
                buf.put_u8(LEGACY_RAW_BYTECODE_ID);
                1
            }
            // [`REMOVED_BYTECODE_ID`] has been removed.
            RevmBytecode::LegacyAnalyzed(analyzed) => {
                buf.put_u8(LEGACY_ANALYZED_BYTECODE_ID);
                buf.put_u64(analyzed.original_len() as u64);
                let map = analyzed.jump_table().as_slice();
                buf.put_slice(map);
                1 + 8 + map.len()
            }
            RevmBytecode::Eof(_) => {
                buf.put_u8(EOF_BYTECODE_ID);
                1
            }
            RevmBytecode::Eip7702(_) => {
                buf.put_u8(EIP7702_BYTECODE_ID);
                1
            }
        };
        len + bytecode.len() + 4
    }

    // # Panics
    //
    // A panic will be triggered if a bytecode variant of 1 or greater than 2 is passed from the
    // database.
    fn from_compact(mut buf: &[u8], _: usize) -> (Self, &[u8]) {
        let len = buf.read_u32::<BigEndian>().expect("could not read bytecode length");
        let bytes = Bytes::from(buf.copy_to_bytes(len as usize));
        let variant = buf.read_u8().expect("could not read bytecode variant");
        let decoded = match variant {
            LEGACY_RAW_BYTECODE_ID => Self(RevmBytecode::new_raw(bytes)),
            REMOVED_BYTECODE_ID => {
                unreachable!("Junk data in database: checked Bytecode variant was removed")
            }
            LEGACY_ANALYZED_BYTECODE_ID => Self(unsafe {
                RevmBytecode::new_analyzed(
                    bytes,
                    buf.read_u64::<BigEndian>().unwrap() as usize,
                    JumpTable::from_slice(buf),
                )
            }),
            EOF_BYTECODE_ID | EIP7702_BYTECODE_ID => {
                // EOF and EIP-7702 bytecode objects will be decoded from the raw bytecode
                Self(RevmBytecode::new_raw(bytes))
            }
            _ => unreachable!("Junk data in database: unknown Bytecode variant"),
        };
        (decoded, &[])
    }
}

impl From<&GenesisAccount> for Account {
    fn from(value: &GenesisAccount) -> Self {
        Self {
            nonce: value.nonce.unwrap_or_default(),
            balance: value.balance,
            bytecode_hash: value.code.as_ref().map(keccak256),
            #[cfg(feature = "scroll")]
            account_extension: Some(reth_scroll_primitives::AccountExtension::from_bytecode(
                value.code.as_ref().unwrap_or_default(),
            )),
        }
    }
}

impl From<AccountInfo> for Account {
    fn from(revm_acc: AccountInfo) -> Self {
        let code_hash = revm_acc.code_hash;
        Self {
            balance: revm_acc.balance,
            nonce: revm_acc.nonce,
            bytecode_hash: (code_hash != KECCAK_EMPTY).then_some(code_hash),
            #[cfg(feature = "scroll")]
            account_extension: Some((revm_acc.code_size, revm_acc.poseidon_code_hash).into()),
        }
    }
}

impl From<Account> for AccountInfo {
    fn from(reth_acc: Account) -> Self {
        Self {
            balance: reth_acc.balance,
            nonce: reth_acc.nonce,
            code_hash: reth_acc.bytecode_hash.unwrap_or(KECCAK_EMPTY),
            code: None,
            #[cfg(feature = "scroll")]
            code_size: reth_acc.account_extension.unwrap_or_default().code_size,
            #[cfg(feature = "scroll")]
            poseidon_code_hash: reth_acc
                .account_extension
                .unwrap_or_default()
                .poseidon_code_hash
                .unwrap_or(reth_scroll_primitives::POSEIDON_EMPTY),
        }
    }
}

#[cfg(feature = "scroll")]
impl From<Account> for revm_primitives::shared::AccountInfo {
    fn from(reth_acc: Account) -> Self {
        Self {
            balance: reth_acc.balance,
            nonce: reth_acc.nonce,
            code_hash: reth_acc.bytecode_hash.unwrap_or(KECCAK_EMPTY),
            code: None,
        }
    }
}

// TODO (scroll): remove at last Scroll `Account` related PR.
#[cfg(feature = "scroll")]
impl From<revm_primitives::shared::AccountInfo> for Account {
    fn from(revm_acc: revm_primitives::shared::AccountInfo) -> Self {
        Self {
            balance: revm_acc.balance,
            nonce: revm_acc.nonce,
            bytecode_hash: (revm_acc.code_hash != KECCAK_EMPTY).then_some(revm_acc.code_hash),
            account_extension: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{hex_literal::hex, B256, U256};
    use revm_primitives::LegacyAnalyzedBytecode;

    #[test]
    fn test_account() {
        let mut buf = vec![];
        let mut acc = Account::default();
        let len = acc.to_compact(&mut buf);
        assert_eq!(len, 2);

        acc.balance = U256::from(2);
        let len = acc.to_compact(&mut buf);
        assert_eq!(len, 3);

        acc.nonce = 2;
        let len = acc.to_compact(&mut buf);
        assert_eq!(len, 4);
    }

    #[test]
    fn test_empty_account() {
        let mut acc =
            Account { nonce: 0, balance: U256::ZERO, bytecode_hash: None, ..Default::default() };
        // Nonce 0, balance 0, and bytecode hash set to None is considered empty.
        assert!(acc.is_empty());

        acc.bytecode_hash = Some(KECCAK_EMPTY);
        // Nonce 0, balance 0, and bytecode hash set to KECCAK_EMPTY is considered empty.
        assert!(acc.is_empty());

        acc.balance = U256::from(2);
        // Non-zero balance makes it non-empty.
        assert!(!acc.is_empty());

        acc.balance = U256::ZERO;
        acc.nonce = 10;
        // Non-zero nonce makes it non-empty.
        assert!(!acc.is_empty());

        acc.nonce = 0;
        acc.bytecode_hash = Some(B256::from(U256::ZERO));
        // Non-empty bytecode hash makes it non-empty.
        assert!(!acc.is_empty());
    }

    #[test]
    fn test_bytecode() {
        let mut buf = vec![];
        let bytecode = Bytecode::new_raw(Bytes::default());
        let len = bytecode.to_compact(&mut buf);
        assert_eq!(len, 5);

        let mut buf = vec![];
        let bytecode = Bytecode::new_raw(Bytes::from(&hex!("ffff")));
        let len = bytecode.to_compact(&mut buf);
        assert_eq!(len, 7);

        let mut buf = vec![];
        let bytecode = Bytecode(RevmBytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(
            Bytes::from(&hex!("ffff")),
            2,
            JumpTable::from_slice(&[0]),
        )));
        let len = bytecode.to_compact(&mut buf);
        assert_eq!(len, 16);

        let (decoded, remainder) = Bytecode::from_compact(&buf, len);
        assert_eq!(decoded, bytecode);
        assert!(remainder.is_empty());
    }

    #[test]
    fn test_account_has_bytecode() {
        // Account with no bytecode (None)
        let acc_no_bytecode = Account {
            nonce: 1,
            balance: U256::from(1000),
            bytecode_hash: None,
            ..Default::default()
        };
        assert!(!acc_no_bytecode.has_bytecode(), "Account should not have bytecode");

        // Account with bytecode hash set to KECCAK_EMPTY (should have bytecode)
        let acc_empty_bytecode = Account {
            nonce: 1,
            balance: U256::from(1000),
            bytecode_hash: Some(KECCAK_EMPTY),
            ..Default::default()
        };
        assert!(acc_empty_bytecode.has_bytecode(), "Account should have bytecode");

        // Account with a non-empty bytecode hash
        let acc_with_bytecode = Account {
            nonce: 1,
            balance: U256::from(1000),
            bytecode_hash: Some(B256::from_slice(&[0x11u8; 32])),
            ..Default::default()
        };
        assert!(acc_with_bytecode.has_bytecode(), "Account should have bytecode");
    }

    #[test]
    fn test_account_get_bytecode_hash() {
        // Account with no bytecode (should return KECCAK_EMPTY)
        let acc_no_bytecode =
            Account { nonce: 0, balance: U256::ZERO, bytecode_hash: None, ..Default::default() };
        assert_eq!(acc_no_bytecode.get_bytecode_hash(), KECCAK_EMPTY, "Should return KECCAK_EMPTY");

        // Account with bytecode hash set to KECCAK_EMPTY
        let acc_empty_bytecode = Account {
            nonce: 1,
            balance: U256::from(1000),
            bytecode_hash: Some(KECCAK_EMPTY),
            ..Default::default()
        };
        assert_eq!(
            acc_empty_bytecode.get_bytecode_hash(),
            KECCAK_EMPTY,
            "Should return KECCAK_EMPTY"
        );

        // Account with a valid bytecode hash
        let bytecode_hash = B256::from_slice(&[0x11u8; 32]);
        let acc_with_bytecode = Account {
            nonce: 1,
            balance: U256::from(1000),
            bytecode_hash: Some(bytecode_hash),
            ..Default::default()
        };
        assert_eq!(
            acc_with_bytecode.get_bytecode_hash(),
            bytecode_hash,
            "Should return the bytecode hash"
        );
    }
}
