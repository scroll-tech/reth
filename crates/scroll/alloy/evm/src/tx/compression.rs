use std::io::Write;

use super::FromRecoveredTx;
use crate::ScrollTransactionIntoTxEnv;
use alloy_consensus::transaction::Recovered;
use alloy_eips::{Encodable2718, Typed2718};
use alloy_evm::{IntoTxEnv, RecoveredTx};
use alloy_primitives::{Address, Bytes, TxKind, U256};
use revm::context::TxEnv;
use revm_scroll::l1block::TX_L1_FEE_PRECISION;
use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};
use zstd::{
    stream::Encoder,
    zstd_safe::{CParameter, ParamSwitch},
};

/// The maximum size of the compression window in bytes (2^CL_WINDOW_LIMIT).
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

/// Computes the compression factor for a given RLP-encoded transaction.
pub fn compute_compression_factor<T: AsRef<[u8]>>(rlp_bytes: &T) -> U256 {
    // Instantiate the compressor
    let mut compressor = compressor(CL_WINDOW_LIMIT);
    let rlp_bytes_len = rlp_bytes.as_ref().len();

    // Set the pledged source size to the length of the RLP bytes and write the bytes to the
    // compressor.
    // TODO: Is it possible this is fallible?
    compressor
        .set_pledged_src_size(Some(rlp_bytes_len as u64))
        .expect("failed to set pledged source size");
    // TODO: Is it possible this is fallible?
    compressor.write_all(rlp_bytes.as_ref()).expect("failed to write RLP bytes to compressor");

    // Finish the compression and get the result.
    let result = compressor.finish().expect("failed to finish compression");

    // compute the compression ratio
    let compression_ratio =
        ((rlp_bytes_len as f64 * TX_L1_FEE_PRECISION as f64) / result.len() as f64).floor() as u64;

    U256::from(compression_ratio)
}

/// A generic wrrapper for a type that includes a compression factor and encoded bytes.
#[derive(Debug, Clone)]
pub struct WithCompression<T> {
    value: T,
    compression_factor: U256,
    encoded_bytes: Bytes,
}

/// A trait for types that can be constructed from a transaction, its sender, encoded bytes and
/// compression factor.
pub trait FromTxWithCompression<Tx> {
    /// Builds a `TxEnv` from a transaction, its sender, encoded transaction bytes, and a
    /// compression factor.
    fn from_compressed_tx(
        tx: &Tx,
        sender: Address,
        encoded: Bytes,
        compression_factor: Option<U256>,
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
        compression_factor: Option<U256>,
    ) -> Self {
        TxEnv::from_compressed_tx(tx, sender, encoded, compression_factor)
    }
}

impl<T, TxEnv: FromTxWithCompression<T>> IntoTxEnv<TxEnv> for WithCompression<Recovered<T>> {
    fn into_tx_env(self) -> TxEnv {
        let recovered = &self.value;
        TxEnv::from_compressed_tx(
            recovered.inner(),
            recovered.signer(),
            self.encoded_bytes.clone(),
            Some(self.compression_factor),
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
            Some(self.compression_factor),
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
            Some(self.compression_factor),
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
            Some(self.compression_factor),
        )
    }
}

impl FromTxWithCompression<ScrollTxEnvelope> for ScrollTransactionIntoTxEnv<TxEnv> {
    fn from_compressed_tx(
        tx: &ScrollTxEnvelope,
        caller: Address,
        encoded: Bytes,
        compression_factor: Option<U256>,
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

        Self::new(base, Some(encoded), compression_factor)
    }
}

/// A trait that allows a type to be converted into [`withCompression`].
pub trait IntoCompressed<T> {
    /// Converts the type into a [`WithCompression`] instance, optionally using a
    /// [`ScrollTxCompressor`] to calculate the compression factor.
    fn into_compressed(&self, compression_factor: U256) -> WithCompression<Recovered<&T>>;
}

impl<T: Encodable2718> IntoCompressed<T> for Recovered<&T> {
    fn into_compressed(&self, compression_factor: U256) -> WithCompression<Recovered<&T>> {
        let encoded_bytes = self.inner().encoded_2718();
        WithCompression { value: *self, compression_factor, encoded_bytes: encoded_bytes.into() }
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
