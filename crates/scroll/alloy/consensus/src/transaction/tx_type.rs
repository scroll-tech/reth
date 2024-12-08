//! Contains the transaction type identifier for Scroll.

use alloy_eips::eip2718::Eip2718Error;
use alloy_primitives::{U64, U8};
use alloy_rlp::{BufMut, Decodable, Encodable};
use derive_more::Display;

/// Identifier for an Scroll L1 message transaction
pub const L1_MESSAGE_TX_TYPE_ID: u8 = 126; // 0x7E

/// Scroll `TransactionType` flags as specified in https://docs.scroll.io/en/technology/chain/transactions/.
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Display)]
pub enum ScrollTxType {
    /// Legacy transaction type.
    #[display("legacy")]
    Legacy = 0,
    /// EIP-2930 transaction type.
    #[display("eip2930")]
    Eip2930 = 1,
    /// EIP-1559 transaction type.
    #[display("eip1559")]
    Eip1559 = 2,
    /// Optimism Deposit transaction type.
    #[display("deposit")]
    L1Message = L1_MESSAGE_TX_TYPE_ID,
}

impl ScrollTxType {
    /// List of all variants.
    pub const ALL: [Self; 2] = [Self::Legacy, Self::L1Message];
}

#[cfg(any(test, feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for ScrollTxType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let i = u.choose_index(Self::ALL.len())?;
        Ok(Self::ALL[i])
    }
}

impl From<ScrollTxType> for U8 {
    fn from(tx_type: ScrollTxType) -> Self {
        Self::from(u8::from(tx_type))
    }
}

impl From<ScrollTxType> for u8 {
    fn from(v: ScrollTxType) -> Self {
        v as Self
    }
}

impl TryFrom<u8> for ScrollTxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Legacy,
            1 => Self::Eip2930,
            2 => Self::Eip1559,
            126 => Self::L1Message,
            _ => return Err(Eip2718Error::UnexpectedType(value)),
        })
    }
}

impl TryFrom<u64> for ScrollTxType {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        let err = || "invalid tx type";
        let value: u8 = value.try_into().map_err(|_| err())?;
        Self::try_from(value).map_err(|_| err())
    }
}

impl TryFrom<U64> for ScrollTxType {
    type Error = &'static str;

    fn try_from(value: U64) -> Result<Self, Self::Error> {
        value.to::<u64>().try_into()
    }
}

impl PartialEq<u8> for ScrollTxType {
    fn eq(&self, other: &u8) -> bool {
        (*self as u8) == *other
    }
}

impl PartialEq<ScrollTxType> for u8 {
    fn eq(&self, other: &ScrollTxType) -> bool {
        *self == *other as Self
    }
}

impl Encodable for ScrollTxType {
    fn encode(&self, out: &mut dyn BufMut) {
        (*self as u8).encode(out);
    }

    fn length(&self) -> usize {
        1
    }
}

impl Decodable for ScrollTxType {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let ty = u8::decode(buf)?;

        Self::try_from(ty).map_err(|_| alloy_rlp::Error::Custom("invalid transaction type"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    #[test]
    fn test_all_tx_types() {
        assert_eq!(ScrollTxType::ALL.len(), 2);
        let all = vec![ScrollTxType::Legacy, ScrollTxType::L1Message];
        assert_eq!(ScrollTxType::ALL.to_vec(), all);
    }

    #[test]
    fn tx_type_roundtrip() {
        for &tx_type in &ScrollTxType::ALL {
            let mut buf = Vec::new();
            tx_type.encode(&mut buf);
            let decoded = ScrollTxType::decode(&mut &buf[..]).unwrap();
            assert_eq!(tx_type, decoded);
        }
    }
}
