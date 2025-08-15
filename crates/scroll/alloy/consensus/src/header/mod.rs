use alloy_consensus::Header;
use alloy_primitives::{
    Address, BlockNumber, Bloom, Bytes, Sealable, B256, B64, U256, keccak256,
};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};
use core::ops::{Deref, DerefMut};

#[cfg(feature = "reth-codec")]
use reth_codecs::Compact;

/// A wrapper around `alloy_consensus::Header` that excludes `extra_data` field when computing hash_slow.
/// 
/// This is useful for Scroll where the `extra_data` field should not be included in the block hash
/// calculation for certain operations.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Default,
    RlpEncodable,
    RlpDecodable,
    Serialize,
    Deserialize,
)]
#[cfg_attr(feature = "reth-codec", derive(Compact))]
#[serde(rename_all = "camelCase")]
pub struct ScrollHeader {
    /// The inner alloy consensus header
    #[serde(flatten)]
    pub inner: Header,
}

impl ScrollHeader {
    /// Create a new ScrollHeader from an Header
    pub fn new(header: Header) -> Self {
        Self { inner: header }
    }

    /// Create a header for hashing by manually encoding all fields except extra_data
    fn encode_for_hashing(&self) -> Vec<u8> {
        let mut out = Vec::new();
        
        // Manually encode each field in the correct order
        self.inner.parent_hash.encode(&mut out);
        self.inner.ommers_hash.encode(&mut out);
        self.inner.beneficiary.encode(&mut out);
        self.inner.state_root.encode(&mut out);
        self.inner.transactions_root.encode(&mut out);
        self.inner.receipts_root.encode(&mut out);
        self.inner.logs_bloom.encode(&mut out);
        self.inner.difficulty.encode(&mut out);
        self.inner.number.encode(&mut out);
        self.inner.gas_limit.encode(&mut out);
        self.inner.gas_used.encode(&mut out);
        self.inner.timestamp.encode(&mut out);
        // Note: extra_data is intentionally excluded
        self.inner.mix_hash.encode(&mut out);
        self.inner.nonce.encode(&mut out);
        
        // Encode optional fields if present
        if let Some(base_fee) = self.inner.base_fee_per_gas {
            base_fee.encode(&mut out);
        }
        if let Some(withdrawals_root) = self.inner.withdrawals_root {
            withdrawals_root.encode(&mut out);
        }
        if let Some(blob_gas_used) = self.inner.blob_gas_used {
            blob_gas_used.encode(&mut out);
        }
        if let Some(excess_blob_gas) = self.inner.excess_blob_gas {
            excess_blob_gas.encode(&mut out);
        }
        if let Some(parent_beacon_block_root) = self.inner.parent_beacon_block_root {
            parent_beacon_block_root.encode(&mut out);
        }
        if let Some(requests_hash) = self.inner.requests_hash {
            requests_hash.encode(&mut out);
        }
        
        out
    }
}

impl From<Header> for ScrollHeader {
    fn from(header: Header) -> Self {
        Self::new(header)
    }
}

impl From<ScrollHeader> for Header {
    fn from(header: ScrollHeader) -> Self {
        header.inner
    }
}

impl AsRef<Header> for ScrollHeader {
    fn as_ref(&self) -> &Header {
        &self.inner
    }
}

impl AsMut<Header> for ScrollHeader {
    fn as_mut(&mut self) -> &mut Header {
        &mut self.inner
    }
}

impl Deref for ScrollHeader {
    type Target = Header;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ScrollHeader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Sealable for ScrollHeader {
    /// Hash the header excluding the extra_data field
    fn hash_slow(&self) -> B256 {
        let encoded = self.encode_for_hashing();
        keccak256(&encoded)
    }
}

impl alloy_consensus::BlockHeader for ScrollHeader {
    fn parent_hash(&self) -> B256 {
        self.inner.parent_hash()
    }

    fn ommers_hash(&self) -> B256 {
        self.inner.ommers_hash()
    }

    fn beneficiary(&self) -> Address {
        self.inner.beneficiary()
    }

    fn state_root(&self) -> B256 {
        self.inner.state_root()
    }

    fn transactions_root(&self) -> B256 {
        self.inner.transactions_root()
    }

    fn receipts_root(&self) -> B256 {
        self.inner.receipts_root()
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.inner.withdrawals_root()
    }

    fn logs_bloom(&self) -> Bloom {
        self.inner.logs_bloom()
    }

    fn difficulty(&self) -> U256 {
        self.inner.difficulty()
    }

    fn number(&self) -> BlockNumber {
        self.inner.number()
    }

    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    fn gas_used(&self) -> u64 {
        self.inner.gas_used()
    }

    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }

    fn mix_hash(&self) -> Option<B256> {
        self.inner.mix_hash()
    }

    fn nonce(&self) -> Option<B64> {
        self.inner.nonce()
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.inner.base_fee_per_gas()
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.inner.blob_gas_used()
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.inner.excess_blob_gas()
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.inner.parent_beacon_block_root()
    }

    fn requests_hash(&self) -> Option<B256> {
        self.inner.requests_hash()
    }

    fn extra_data(&self) -> &Bytes {
        self.inner.extra_data()
    }
}

// Additional trait implementations required by reth_primitives_traits::BlockHeader
impl reth_primitives_traits::block::header::BlockHeaderMut for ScrollHeader {
    fn extra_data_mut(&mut self) -> &mut Bytes {
        &mut self.inner.extra_data
    }
}

impl reth_primitives_traits::InMemorySize for ScrollHeader {
    fn size(&self) -> usize {
        self.inner.size()
    }
}

impl AsRef<Self> for ScrollHeader {
    fn as_ref(&self) -> &Self {
        self
    }
}

// Implement reth_primitives_traits::BlockHeader
impl reth_primitives_traits::BlockHeader for ScrollHeader {}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Bytes;

    #[test]
    fn test_hash_excludes_extra_data() {
        // Create two identical headers with different extra_data
        let mut header1 = Header::default();
        let mut header2 = Header::default();
        
        header1.extra_data = Bytes::from("test_data_1");
        header2.extra_data = Bytes::from("test_data_2");
        
        let scroll_header1 = ScrollHeader::new(header1.clone());
        let scroll_header2 = ScrollHeader::new(header2.clone());

        // The hashes should be the same since extra_data is excluded
        assert_eq!(scroll_header1.hash_slow(), scroll_header2.hash_slow());
        
        // But the original alloy headers would have different hashes
        assert_ne!(header1.hash_slow(), header2.hash_slow());
    }

    #[test]
    fn test_different_fields_produce_different_hashes() {
        let mut header1 = Header::default();
        let mut header2 = Header::default();
        
        header1.number = 1;
        header2.number = 2;
        
        let scroll_header1 = ScrollHeader::new(header1);
        let scroll_header2 = ScrollHeader::new(header2);
        
        // Different numbers should produce different hashes
        assert_ne!(scroll_header1.hash_slow(), scroll_header2.hash_slow());
    }
}