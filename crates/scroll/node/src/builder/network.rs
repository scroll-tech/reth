use alloy_primitives::{Address, Signature, B256};
use async_trait::async_trait;
use reth_chainspec::EthChainSpec;
use reth_eth_wire_types::BasicNetworkPrimitives;
use reth_network::{
    config::NetworkMode,
    protocol::{RlpxSubProtocol, RlpxSubProtocols},
    transform::header::HeaderTransform,
    NetworkConfig, NetworkHandle, NetworkManager, PeersInfo,
};
use reth_node_api::TxTy;
use reth_node_builder::{components::NetworkBuilder, BuilderContext, FullNodeTypes};
use reth_node_types::NodeTypes;
use reth_primitives_traits::BlockHeader;
use reth_scroll_chainspec::ScrollChainSpec;
use reth_scroll_primitives::ScrollPrimitives;
use reth_tracing::tracing::{debug, info, trace, warn};
use reth_transaction_pool::{PoolTransaction, TransactionPool};
use scroll_alloy_hardforks::ScrollHardforks;
use std::{fmt, fmt::Debug, sync::Arc};

/// Errors that can occur during signature validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeaderTransformError {
    /// Invalid signature length (expected 65 bytes)
    InvalidSignature,
    /// Invalid signer (not authorized)
    InvalidSigner(Address),
    /// Signature recovery failed
    RecoveryFailed,
    /// No tokio runtime available
    NoRuntimeAvailable,
    /// Database operation failed
    DatabaseError(String),
}

impl fmt::Display for HeaderTransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature length, expected 65 bytes"),
            Self::InvalidSigner(signer) => write!(f, "Invalid signer, not authorized: {}", signer),
            Self::RecoveryFailed => write!(f, "Failed to recover signer from signature"),
            Self::NoRuntimeAvailable => {
                write!(f, "No tokio runtime available during signature storage")
            }
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for HeaderTransformError {}

/// The network builder for Scroll.
#[derive(Debug, Default)]
pub struct ScrollNetworkBuilder {
    /// Additional `RLPx` sub-protocols to be added to the network.
    scroll_sub_protocols: RlpxSubProtocols,
}

impl ScrollNetworkBuilder {
    /// Create a new [`ScrollNetworkBuilder`] with default configuration.
    pub fn new() -> Self {
        Self { scroll_sub_protocols: RlpxSubProtocols::default() }
    }

    /// Add a scroll sub-protocol to the network builder.
    pub fn with_sub_protocol(mut self, protocol: RlpxSubProtocol) -> Self {
        self.scroll_sub_protocols.push(protocol);
        self
    }
}

impl<Node, Pool> NetworkBuilder<Node, Pool> for ScrollNetworkBuilder
where
    Node:
        FullNodeTypes<Types: NodeTypes<ChainSpec = ScrollChainSpec, Primitives = ScrollPrimitives>>,
    Pool: TransactionPool<
            Transaction: PoolTransaction<
                Consensus = TxTy<Node::Types>,
                Pooled = scroll_alloy_consensus::ScrollPooledTransaction,
            >,
        > + Unpin
        + 'static,
{
    type Network = NetworkHandle<ScrollNetworkPrimitives>;

    async fn build_network(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<Self::Network> {
        // set the network mode to work.
        let config = ctx.network_config()?;
        let config = NetworkConfig {
            network_mode: NetworkMode::Work,
            extra_protocols: self.scroll_sub_protocols,
            ..config
        };

        let network = NetworkManager::builder(config).await?;
        let handle = ctx.start_network(network, pool, None);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

/// Network primitive types used by Scroll networks.
pub type ScrollNetworkPrimitives =
    BasicNetworkPrimitives<ScrollPrimitives, scroll_alloy_consensus::ScrollPooledTransaction>;

/// A trait for getting and inserting signatures from a database.
#[async_trait]
pub trait SignatureProvider {
    /// The error type returned by signature operations.
    type Error: std::fmt::Debug + std::fmt::Display + Send;

    /// Retrieves a signature for the given block hash.
    async fn get_signature(&self, hash: B256) -> Result<Option<Signature>, Self::Error>;

    /// Inserts a signature for the given block hash.
    async fn insert_signature(&self, hash: B256, signature: Signature) -> Result<(), Self::Error>;
}

/// An implementation of a [`HeaderTransform`] for downloaded headers for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollHeaderTransform<ChainSpec, P> {
    chain_spec: ChainSpec,
    provider: Arc<P>,
    signer: Option<Address>,
}

impl<
        ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync + 'static,
        P: SignatureProvider + Debug + Send + Sync + 'static,
    > ScrollHeaderTransform<ChainSpec, P>
{
    /// Returns a new instance of the [`ScrollHeaderTransform`] from the provider chain spec.
    pub const fn new(chain_spec: ChainSpec, provider: Arc<P>, signer: Option<Address>) -> Self {
        Self { chain_spec, provider, signer }
    }

    /// Returns a new [`ScrollHeaderTransform`] as a [`HeaderTransform`] trait object.
    pub fn boxed<H: BlockHeader>(
        chain_spec: ChainSpec,
        provider: Arc<P>,
        signer: Option<Address>,
    ) -> Box<dyn HeaderTransform<H>> {
        Box::new(Self { chain_spec, provider, signer })
    }
}

impl<
        H: BlockHeader,
        ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync,
        P: SignatureProvider + Debug + Send + Sync + 'static,
    > HeaderTransform<H> for ScrollHeaderTransform<ChainSpec, P>
{
    fn map(&self, mut header: H) -> H {
        if !self.chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
            return header;
        }
        // TODO: remove this once we deprecated l2geth
        // Validate and process signature

        if let Err(err) = self.validate_and_store_signature(&mut header, self.signer) {
            debug!(
                target: "scroll::network::response_header_transform",
                "Header signature persistence failed, header hash: {:?}, error: {}",
                header.hash_slow(), err
            );
        }

        header
    }
}

impl<
        ChainSpec: ScrollHardforks + Debug + Send + Sync,
        P: SignatureProvider + Debug + Send + Sync + 'static,
    > ScrollHeaderTransform<ChainSpec, P>
{
    fn validate_and_store_signature<H: BlockHeader>(
        &self,
        header: &mut H,
        authorized_signer: Option<Address>,
    ) -> Result<(), HeaderTransformError> {
        let signature_bytes = std::mem::take(header.extra_data_mut());
        let signature = parse_65b_signature(&signature_bytes)?;

        // Recover and verify signer
        recover_and_verify_signer(&signature, header.hash_slow(), authorized_signer)?;

        // Store signature in database
        persist_signature(self.provider.clone(), header.hash_slow(), signature);

        Ok(())
    }
}

/// An implementation of a [`HeaderTransform`] for header request responses for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollRequestHeaderTransform<ChainSpec, P> {
    chain_spec: ChainSpec,
    provider: Arc<P>,
    signer: Option<Address>,
}

impl<
        ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync + 'static,
        P: SignatureProvider + Debug + Send + Sync + 'static,
    > ScrollRequestHeaderTransform<ChainSpec, P>
{
    /// Returns a new instance of the [`ScrollHeaderTransform`] from the provider chain spec.
    pub const fn new(chain_spec: ChainSpec, provider: Arc<P>, signer: Option<Address>) -> Self {
        Self { chain_spec, provider, signer }
    }
}

impl<
        H: BlockHeader,
        ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync,
        P: SignatureProvider + Debug + Send + Sync,
    > HeaderTransform<H> for ScrollRequestHeaderTransform<ChainSpec, P>
{
    fn map(&self, mut header: H) -> H {
        if !self.chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
            return header;
        }

        // read the signature from the rollup node.
        let signature = tokio::task::block_in_place(|| {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                match handle
                    .block_on(async { self.provider.get_signature(header.hash_slow()).await })
                {
                    Ok(sig) => sig,
                    Err(e) => {
                        warn!(target: "scroll::network::request_header_transform",
                            "Failed to get block signature from database, header hash: {:?}, error: {}",
                            header.hash_slow(),
                            HeaderTransformError::DatabaseError(e.to_string())
                        );
                        None
                    }
                }
            } else {
                warn!(
                    target: "scroll::network::request_header_transform",
                    "Failed to get block signature from database, header hash: {:?}, error: {}",
                    header.hash_slow(),
                    HeaderTransformError::NoRuntimeAvailable
                );
                None
            }
        });

        // If we have a signature in the database and it matches configured signer then add it
        // to the extra data field
        if let Some(sig) = signature {
            if let Err(err) = recover_and_verify_signer(&sig, header.hash_slow(), self.signer) {
                warn!(
                target: "scroll::network::request_header_transform",
                    "Found invalid signature(different from the hardcoded signer) for header hash: {:?}, sig: {:?}, error: {}",
                    header.hash_slow(),
                    sig.to_string(),
                    err
                );
            } else {
                *header.extra_data_mut() = sig.as_bytes().into();
            }
        }

        header
    }
}

/// Recover signer from signature and verify authorization.
fn recover_and_verify_signer(
    signature: &Signature,
    hash: B256,
    authorized_signer: Option<Address>,
) -> Result<Address, HeaderTransformError> {
    // Recover signer from signature
    let signer = reth_primitives_traits::crypto::secp256k1::recover_signer(signature, hash)
        .map_err(|_| HeaderTransformError::RecoveryFailed)?;

    // Verify signer is authorized
    if Some(signer) != authorized_signer {
        return Err(HeaderTransformError::InvalidSigner(signer));
    }

    Ok(signer)
}

/// Parse a canonical 65-byte secp256k1 signature: r (32) | s (32) | v (1).
fn parse_65b_signature(bytes: &[u8]) -> Result<Signature, HeaderTransformError> {
    if bytes.len() != 65 {
        return Err(HeaderTransformError::InvalidSignature);
    }

    let signature =
        Signature::from_raw(bytes).map_err(|_| HeaderTransformError::InvalidSignature)?;

    Ok(signature)
}

/// Run the async DB insert from sync code safely.
fn persist_signature<P: SignatureProvider + Send + Sync + 'static>(
    provider: Arc<P>,
    hash: B256,
    signature: Signature,
) {
    tokio::spawn(async move {
        trace!(
            "Persisting block signature to database, block hash: {:?}, sig: {:?}",
            hash,
            signature.to_string()
        );
        if let Err(e) = provider.insert_signature(hash, signature).await {
            warn!(target: "scroll::network::header_transform", "Failed to store signature in database: {:?}", e);
        }
    });
}
