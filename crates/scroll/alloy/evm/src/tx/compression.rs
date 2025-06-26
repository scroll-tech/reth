use super::FromRecoveredTx;
use crate::ScrollTransactionIntoTxEnv;
use alloy_consensus::transaction::Recovered;
use alloy_eips::{Encodable2718, Typed2718};
use alloy_evm::{IntoTxEnv, RecoveredTx};
use alloy_primitives::{Address, Bytes, TxKind, U256};
use revm::context::TxEnv;
use revm_scroll::l1block::TX_L1_FEE_PRECISION_U256;
use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};
pub use zstd_compression::compute_compression_ratio;

#[cfg(feature = "zstd_compression")]
mod zstd_compression {
    use super::*;
    use std::io::Write;
    use zstd::{
        stream::Encoder,
        zstd_safe::{CParameter, ParamSwitch},
    };

    /// The maximum size of the compression window in bytes (`2^CL_WINDOW_LIMIT`).
    const CL_WINDOW_LIMIT: u32 = 22;

    fn compressor(target_block_size: u32) -> Encoder<'static, Vec<u8>> {
        let mut encoder = Encoder::new(Vec::new(), 0).expect("Failed to create zstd encoder");
        encoder
            .set_parameter(CParameter::LiteralCompressionMode(ParamSwitch::Disable))
            .expect("Failed to set literal compression mode");
        encoder
            .set_parameter(CParameter::WindowLog(CL_WINDOW_LIMIT))
            .expect("Failed to set window log");
        encoder
            .set_parameter(CParameter::TargetCBlockSize(target_block_size))
            .expect("Failed to set target block size");
        encoder.include_checksum(false).expect("Failed to disable checksum");
        encoder.include_magicbytes(false).expect("Failed to disable magic bytes");
        encoder.include_dictid(false).expect("Failed to disable dictid");
        encoder.include_contentsize(true).expect("Failed to include content size");
        encoder
    }

    /// Computes the compression ratio for the provided bytes.
    ///
    /// This is computed as:
    /// `(original_size * TX_L1_FEE_PRECISION_U256) / compressed_size`
    pub fn compute_compression_ratio<T: AsRef<[u8]>>(bytes: &T) -> U256 {
        // Instantiate the compressor
        let mut compressor = compressor(CL_WINDOW_LIMIT);
        let original_bytes_len = bytes.as_ref().len();

        // Set the pledged source size to the length of the bytes and write the bytes to the
        // compressor.
        compressor
            .set_pledged_src_size(Some(original_bytes_len as u64))
            .expect("failed to set pledged source size");
        compressor.write_all(bytes.as_ref()).expect("failed to write bytes to compressor");

        // Finish the compression and get the result.
        let result = compressor.finish().expect("failed to finish compression");

        // compute the compression ratio
        let original_len = U256::from(original_bytes_len).saturating_mul(TX_L1_FEE_PRECISION_U256);
        let compressed_len = U256::from(result.len());
        original_len.wrapping_div(compressed_len)
    }
}

#[cfg(not(feature = "zstd_compression"))]
mod zstd_compression {
    use super::*;

    /// Computes the compression ratio for the provided bytes. This panics if the compression
    /// feature is not enabled. This is to support `no_std` environments where zstd is not
    /// available.
    pub fn compute_compression_ratio<T: AsRef<[u8]>>(_bytes: &T) -> U256 {
        panic!("Compression feature is not enabled. Please enable the 'compression' feature to use this function.");
    }
}

/// A generic wrapper for a type that includes a compression ratio and encoded bytes.
#[derive(Debug, Clone)]
pub struct WithCompression<T> {
    value: T,
    compression_ratio: U256,
    encoded_bytes: Bytes,
}

/// A trait for types that can be constructed from a transaction, its sender, encoded bytes and
/// compression ratio.
pub trait FromTxWithCompression<Tx> {
    /// Builds a `TxEnv` from a transaction, its sender, encoded transaction bytes, and a
    /// compression ratio.
    fn from_compressed_tx(
        tx: &Tx,
        sender: Address,
        encoded: Bytes,
        compression_ratio: Option<U256>,
    ) -> Self;
}

impl<TxEnv, T> FromTxWithCompression<&T> for TxEnv
where
    TxEnv: FromTxWithCompression<T>,
{
    fn from_compressed_tx(
        tx: &&T,
        sender: Address,
        encoded: Bytes,
        compression_ratio: Option<U256>,
    ) -> Self {
        TxEnv::from_compressed_tx(tx, sender, encoded, compression_ratio)
    }
}

impl<T, TxEnv: FromTxWithCompression<T>> IntoTxEnv<TxEnv> for WithCompression<Recovered<T>> {
    fn into_tx_env(self) -> TxEnv {
        let recovered = &self.value;
        TxEnv::from_compressed_tx(
            recovered.inner(),
            recovered.signer(),
            self.encoded_bytes.clone(),
            Some(self.compression_ratio),
        )
    }
}

impl<T, TxEnv: FromTxWithCompression<T>> IntoTxEnv<TxEnv> for &WithCompression<Recovered<T>> {
    fn into_tx_env(self) -> TxEnv {
        let recovered = &self.value;
        TxEnv::from_compressed_tx(
            recovered.inner(),
            recovered.signer(),
            self.encoded_bytes.clone(),
            Some(self.compression_ratio),
        )
    }
}

impl<T, TxEnv: FromTxWithCompression<T>> IntoTxEnv<TxEnv> for WithCompression<&Recovered<T>> {
    fn into_tx_env(self) -> TxEnv {
        let recovered = &self.value;
        TxEnv::from_compressed_tx(
            recovered.inner(),
            *recovered.signer(),
            self.encoded_bytes.clone(),
            Some(self.compression_ratio),
        )
    }
}

impl<T, TxEnv: FromTxWithCompression<T>> IntoTxEnv<TxEnv> for &WithCompression<&Recovered<T>> {
    fn into_tx_env(self) -> TxEnv {
        let recovered = &self.value;
        TxEnv::from_compressed_tx(
            recovered.inner(),
            *recovered.signer(),
            self.encoded_bytes.clone(),
            Some(self.compression_ratio),
        )
    }
}

impl FromTxWithCompression<ScrollTxEnvelope> for ScrollTransactionIntoTxEnv<TxEnv> {
    fn from_compressed_tx(
        tx: &ScrollTxEnvelope,
        caller: Address,
        encoded: Bytes,
        compression_ratio: Option<U256>,
    ) -> Self {
        let base = match &tx {
            ScrollTxEnvelope::Legacy(tx) => TxEnv::from_recovered_tx(tx.tx(), caller),
            ScrollTxEnvelope::Eip2930(tx) => TxEnv::from_recovered_tx(tx.tx(), caller),
            ScrollTxEnvelope::Eip1559(tx) => TxEnv::from_recovered_tx(tx.tx(), caller),
            ScrollTxEnvelope::Eip7702(tx) => TxEnv::from_recovered_tx(tx.tx(), caller),
            ScrollTxEnvelope::L1Message(tx) => {
                let TxL1Message { to, value, gas_limit, input, queue_index: _, sender: _ } = &**tx;
                TxEnv {
                    tx_type: tx.ty(),
                    caller,
                    gas_limit: *gas_limit,
                    kind: TxKind::Call(*to),
                    value: *value,
                    data: input.clone(),
                    ..Default::default()
                }
            }
        };

        Self::new(base, Some(encoded), compression_ratio)
    }
}

/// A trait that allows a type to be converted into [`WithCompression`].
pub trait ToCompressed<T> {
    /// Converts the type into a [`WithCompression`] instance using the provided compression ratio.
    fn to_compressed(&self, compression_ratio: U256) -> WithCompression<Recovered<&T>>;
}

impl<T: Encodable2718> ToCompressed<T> for Recovered<&T> {
    fn to_compressed(&self, compression_ratio: U256) -> WithCompression<Recovered<&T>> {
        let encoded_bytes = self.inner().encoded_2718();
        WithCompression { value: *self, compression_ratio, encoded_bytes: encoded_bytes.into() }
    }
}

impl<Tx, T: RecoveredTx<Tx>> RecoveredTx<Tx> for WithCompression<T> {
    fn tx(&self) -> &Tx {
        self.value.tx()
    }

    fn signer(&self) -> &Address {
        self.value.signer()
    }
}
