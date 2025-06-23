use std::collections::HashMap;

use alloy_consensus::transaction::Recovered;
use alloy_eips::Typed2718;
use alloy_evm::{IntoTxEnv, RecoveredTx};
// use alloy_consensus::transaction::Recovered;
// use alloy_evm::IntoTxEnv;
use super::FromRecoveredTx;
use crate::ScrollTransactionIntoTxEnv;
use alloy_primitives::{Address, Bytes, TxKind, B256, U256};
use revm::context::TxEnv;
use scroll_alloy_consensus::{ScrollTxEnvelope, TxL1Message};

/// A cache for transaction compression factors, mapping transaction hashes to their compression
/// factors.
pub type ScrollTxCompressionFactorCache = HashMap<B256, U256>;

const TX_L1_FEE_PRECISION: U256 = U256::from_limbs([1_000_000_000u64, 0, 0, 0]);

/// Computes the compression factor for a given RLP-encoded transaction.
pub fn compute_compression_factor<T: AsRef<[u8]>>(_rlp_bytes: &T) -> U256 {
    U256::from(10).saturating_mul(TX_L1_FEE_PRECISION)
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
    fn into_compressed(
        self,
        compression_provider: Option<&mut ScrollTxCompressionFactorCache>,
    ) -> WithCompression<T>;
}

impl<Tx, T: RecoveredTx<Tx>> RecoveredTx<Tx> for WithCompression<T> {
    fn tx(&self) -> &Tx {
        self.value.tx()
    }

    fn signer(&self) -> &Address {
        self.value.signer()
    }
}
