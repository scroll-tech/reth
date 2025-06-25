use super::FromRecoveredTx;
use crate::ScrollTransactionIntoTxEnv;
use alloy_consensus::transaction::Recovered;
use alloy_eips::{Encodable2718, Typed2718};
use alloy_evm::{IntoTxEnv, RecoveredTx};
use alloy_primitives::{Address, Bytes, TxKind, U256};
use revm::context::TxEnv;
use revm_scroll::l1block::TX_L1_FEE_PRECISION;
use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};
use zstd_safe::{compress_bound, CCtx, CParameter, FrameFormat, ParamSwitch};

/// The maximum size of the compression window in bytes (`2^CL_WINDOW_LIMIT`).
const CL_WINDOW_LIMIT: u32 = 22;

/// The compression level used for zstd compression.
const COMPRESSION_LEVEL: i32 = 3;

/// Creates a zstd compressor with the specified target block size.
pub fn compressor_zstd(target_block_size: u32) -> CCtx<'static> {
    let mut ctx = CCtx::default();
    ctx.set_parameter(CParameter::LiteralCompressionMode(ParamSwitch::Disable))
        .expect("Failed to set literal compression mode");
    ctx.set_parameter(CParameter::WindowLog(CL_WINDOW_LIMIT)).expect("Failed to set window log");
    ctx.set_parameter(CParameter::TargetCBlockSize(target_block_size))
        .expect("Failed to set target block size");
    ctx.set_parameter(CParameter::ChecksumFlag(false)).expect("Failed to set checksum flag");
    ctx.set_parameter(CParameter::Format(FrameFormat::Magicless))
        .expect("msg: Failed to set frame format");
    ctx.set_parameter(CParameter::DictIdFlag(false)).expect("Failed to set dictid flag");
    ctx.set_parameter(CParameter::ContentSizeFlag(true)).expect("Failed to set content size flag");
    ctx
}

/// Compresses the input data using zstd compression with a target block size of
/// [`CL_WINDOW_LIMIT`].
pub fn compress_zstd<T: AsRef<[u8]>>(input: &T) -> Vec<u8> {
    let mut compressor = compressor_zstd(CL_WINDOW_LIMIT);
    let max_compressed_size = compress_bound(input.as_ref().len());
    let mut output_buf = vec![0u8; max_compressed_size];
    let output_slice = &mut output_buf[..];
    let compressed_bytes = compressor
        .compress(output_slice, input.as_ref(), COMPRESSION_LEVEL)
        .expect("Failed to compress data");
    output_buf.truncate(compressed_bytes);
    output_buf
}

/// Computes the compression ratio of the bytes after compressing them with zstd.
pub fn compute_zstd_compression_ratio<T: AsRef<[u8]>>(bytes: &T) -> U256 {
    // Compress the bytes
    let compressed_bytes = compress_zstd(bytes);
    let bytes_len = bytes.as_ref().len();

    // Compute the compression ratio
    let compression_ratio = ((bytes_len as f64 * TX_L1_FEE_PRECISION as f64) /
        compressed_bytes.len() as f64)
        .floor() as u64;

    U256::from(compression_ratio)
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
