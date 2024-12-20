//! Scroll L1 message transaction

use alloy_consensus::{Sealable, Transaction};
use alloy_primitives::{
    keccak256,
    private::alloy_rlp::{Encodable, Header},
    Address, Bytes, ChainId, PrimitiveSignature as Signature, TxHash, TxKind, B256, U256,
};
use alloy_rlp::Decodable;

/// L1 message transaction type id, 0x7e in hex.
const L1_MESSAGE_TRANSACTION_TYPE: u8 = 126;

/// A message transaction sent from the settlement layer to the L2 for execution.
///
/// The signature of the L1 message is already verified on the L1 and as such doesn't contain
/// a signature field. Gas for the transaction execution on Scroll is already paid for on the L1.
///
/// # Bincode compatibility
///
/// `bincode` crate doesn't work with optionally serializable serde fields and some of the execution
/// types require optional serialization for RPC compatibility. Since `TxL1Message` doesn't
/// contain optionally serializable fields, no `bincode` compatible bridge implementation is
/// required.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TxL1Message {
    /// The queue index of the message in the L1 contract queue.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub queue_index: u64,
    /// The gas limit for the transaction. Gas is paid for when message is sent from the L1.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity", rename = "gas"))]
    pub gas_limit: u64,
    /// The destination for the transaction. `Address` is used in place of `TxKind` since contract
    /// creations aren't allowed via L1 message transactions.
    pub to: Address,
    /// The value sent.
    pub value: U256,
    /// The L1 sender of the transaction.
    pub sender: Address,
    /// The input of the transaction.
    pub input: Bytes,
}

impl TxL1Message {
    /// Returns an empty signature for the [`TxL1Message`], which don't include a signature.
    pub fn signature() -> Signature {
        Signature::new(U256::ZERO, U256::ZERO, false)
    }

    /// Decodes the inner [`TxL1Message`] fields from RLP bytes.
    ///
    /// NOTE: This assumes a RLP header has already been decoded, and _just_ decodes the following
    /// RLP fields in the following order:
    ///
    /// - `queue_index`
    /// - `gas_limit`
    /// - `to`
    /// - `value`
    /// - `input`
    /// - `sender`
    pub fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            queue_index: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            to: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            sender: Decodable::decode(buf)?,
        })
    }

    /// Decodes the transaction from RLP bytes.
    pub fn rlp_decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let remaining = buf.len();

        let this = Self::rlp_decode_fields(buf)?;

        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: header.payload_length,
                got: remaining - buf.len(),
            });
        }

        Ok(this)
    }

    /// Outputs the length of the transaction's fields, without a RLP header.
    fn rlp_encoded_fields_length(&self) -> usize {
        self.queue_index.length() +
            self.gas_limit.length() +
            self.to.length() +
            self.value.length() +
            self.input.0.length() +
            self.sender.length()
    }

    /// Encode the fields of the transaction without a RLP header.
    /// <https://github.com/scroll-tech/go-ethereum/blob/9fff27e4f34fb5097100ed76ee725ce056267f4b/core/types/l1_message_tx.go#L12-L19>
    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.queue_index.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.encode(out);
        self.sender.encode(out);
    }

    /// Create a RLP header for the transaction.
    fn rlp_header(&self) -> Header {
        Header { list: true, payload_length: self.rlp_encoded_fields_length() }
    }

    /// RLP encodes the transaction.
    pub fn rlp_encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.rlp_header().encode(out);
        self.rlp_encode_fields(out);
    }

    /// Get the length of the transaction when RLP encoded.
    pub fn rlp_encoded_length(&self) -> usize {
        self.rlp_header().length_with_payload()
    }

    /// Get the length of the transaction when EIP-2718 encoded. This is the
    /// 1 byte type flag + the length of the RLP encoded transaction.
    pub fn eip2718_encoded_length(&self) -> usize {
        self.rlp_encoded_length() + 1
    }

    /// EIP-2718 encode the transaction.
    pub fn eip2718_encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(L1_MESSAGE_TRANSACTION_TYPE);
        self.rlp_encode(out)
    }

    /// Calculates the in-memory size of the [`TxL1Message`] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        size_of::<u64>() + // queue_index
            size_of::<u64>() + // gas_limit
            size_of::<Address>() + // to
            size_of::<U256>() + // value
            self.input.len() + // input
            size_of::<Address>() // sender
    }

    /// Calculates the hash of the [`TxL1Message`] transaction.
    pub fn tx_hash(&self) -> TxHash {
        let mut buf = Vec::with_capacity(self.eip2718_encoded_length());
        self.eip2718_encode(&mut buf);
        keccak256(&buf)
    }
}

impl Transaction for TxL1Message {
    fn chain_id(&self) -> Option<ChainId> {
        None
    }

    fn nonce(&self) -> u64 {
        0u64
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_gas(&self) -> u128 {
        0
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        None
    }

    fn priority_fee_or_price(&self) -> u128 {
        0
    }

    fn effective_gas_price(&self, _base_fee: Option<u64>) -> u128 {
        0
    }

    fn is_dynamic_fee(&self) -> bool {
        false
    }

    fn kind(&self) -> TxKind {
        TxKind::Call(self.to)
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &Bytes {
        &self.input
    }

    fn ty(&self) -> u8 {
        L1_MESSAGE_TRANSACTION_TYPE
    }

    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        None
    }

    fn authorization_list(&self) -> Option<&[alloy_eips::eip7702::SignedAuthorization]> {
        None
    }
}

impl Encodable for TxL1Message {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.rlp_encode(out)
    }

    fn length(&self) -> usize {
        self.rlp_encoded_length()
    }
}

impl Decodable for TxL1Message {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode(buf)
    }
}

impl Sealable for TxL1Message {
    fn hash_slow(&self) -> B256 {
        self.tx_hash()
    }
}

/// Deposit transactions don't have a signature, however, we include an empty signature in the
/// response for better compatibility.
///
/// This function can be used as `serialize_with` serde attribute for the [`TxL1Message`] and will
/// flatten [`TxL1Message::signature`] into response.
///
/// <https://github.com/scroll-tech/go-ethereum/blob/develop/core/types/l1_message_tx.go#L51>.
#[cfg(feature = "serde")]
pub fn serde_l1_message_tx_rpc<T: serde::Serialize, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct SerdeHelper<'a, T> {
        #[serde(flatten)]
        value: &'a T,
        #[serde(flatten)]
        signature: Signature,
    }

    SerdeHelper { value, signature: TxL1Message::signature() }.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use alloy_rlp::BytesMut;

    #[test]
    fn test_rlp_roundtrip() {
        let bytes = Bytes::from_static(&hex!("7ef9015aa044bae9d41b8380d781187b426c6fe43df5fb2fb57bd4466ef6a701e1f01e015694deaddeaddeaddeaddeaddeaddeaddeaddead000194420000000000000000000000000000000000001580808408f0d18001b90104015d8eb900000000000000000000000000000000000000000000000000000000008057650000000000000000000000000000000000000000000000000000000063d96d10000000000000000000000000000000000000000000000000000000000009f35273d89754a1e0387b89520d989d3be9c37c1f32495a88faf1ea05c61121ab0d1900000000000000000000000000000000000000000000000000000000000000010000000000000000000000002d679b567db6187c0c8323fa982cfb88b74dbcc7000000000000000000000000000000000000000000000000000000000000083400000000000000000000000000000000000000000000000000000000000f4240"));
        let tx_a = TxL1Message::decode(&mut bytes[1..].as_ref()).unwrap();
        let mut buf_a = BytesMut::default();
        tx_a.encode(&mut buf_a);
        assert_eq!(&buf_a[..], &bytes[1..]);
    }

    #[test]
    fn test_encode_decode_fields() {
        let original = TxL1Message {
            queue_index: 0,
            gas_limit: 0,
            to: Address::default(),
            value: U256::default(),
            sender: Address::default(),
            input: Bytes::default(),
        };

        let mut buffer = BytesMut::new();
        original.rlp_encode_fields(&mut buffer);
        let decoded = TxL1Message::rlp_decode_fields(&mut &buffer[..]).expect("Failed to decode");

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_with_and_without_header() {
        let tx_deposit = TxL1Message {
            queue_index: 0,
            gas_limit: 50000,
            to: Address::default(),
            value: U256::default(),
            sender: Address::default(),
            input: Bytes::default(),
        };

        let mut buffer_with_header = BytesMut::new();
        tx_deposit.encode(&mut buffer_with_header);

        let mut buffer_without_header = BytesMut::new();
        tx_deposit.rlp_encode_fields(&mut buffer_without_header);

        assert!(buffer_with_header.len() > buffer_without_header.len());
    }

    #[test]
    fn test_payload_length() {
        let tx_deposit = TxL1Message {
            queue_index: 0,
            gas_limit: 50000,
            to: Address::default(),
            value: U256::default(),
            sender: Address::default(),
            input: Bytes::default(),
        };

        assert!(tx_deposit.size() > tx_deposit.rlp_encoded_fields_length());
    }
}

/// Bincode-compatible [`TxL1Message`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(super) mod serde_bincode_compat {
    extern crate alloc;
    use alloc::borrow::Cow;
    use alloy_primitives::{Address, Bytes, U256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::TxL1Message`] serde implementation.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TxL1Message<'a> {
        #[serde(default)]
        queue_index: u64,
        #[serde(default)]
        gas_limit: u64,
        to: Address,
        value: U256,
        sender: Address,
        input: Cow<'a, Bytes>,
    }

    impl<'a> From<&'a super::TxL1Message> for TxL1Message<'a> {
        fn from(value: &'a super::TxL1Message) -> Self {
            Self {
                queue_index: value.queue_index,
                gas_limit: value.gas_limit,
                to: value.to,
                value: value.value,
                sender: value.sender,
                input: Cow::Borrowed(&value.input),
            }
        }
    }

    impl<'a> From<TxL1Message<'a>> for super::TxL1Message {
        fn from(value: TxL1Message<'a>) -> Self {
            Self {
                queue_index: value.queue_index,
                gas_limit: value.gas_limit,
                to: value.to,
                value: value.value,
                sender: value.sender,
                input: value.input.into_owned(),
            }
        }
    }

    impl SerializeAs<super::TxL1Message> for TxL1Message<'_> {
        fn serialize_as<S>(source: &super::TxL1Message, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            TxL1Message::from(source).serialize(serializer)
        }
    }

    impl<'de> DeserializeAs<'de, super::TxL1Message> for TxL1Message<'de> {
        fn deserialize_as<D>(deserializer: D) -> Result<super::TxL1Message, D::Error>
        where
            D: Deserializer<'de>,
        {
            TxL1Message::deserialize(deserializer).map(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use arbitrary::Arbitrary;
        use rand::Rng;
        use serde::{Deserialize, Serialize};
        use serde_with::serde_as;

        use super::super::{serde_bincode_compat, TxL1Message};

        #[test]
        fn test_tx_deposit_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data {
                #[serde_as(as = "serde_bincode_compat::TxDeposit")]
                transaction: TxL1Message,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let data = Data {
                transaction: TxL1Message::arbitrary(&mut arbitrary::Unstructured::new(&bytes))
                    .unwrap(),
            };

            let encoded = bincode::serialize(&data).unwrap();
            let decoded: Data = bincode::deserialize(&encoded).unwrap();
            assert_eq!(decoded, data);
        }
    }
}
