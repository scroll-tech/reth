use alloy_primitives::{address, Address, Signature, B256};
use reth_chainspec::{EthChainSpec, NamedChain};
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
use scroll_rollup_node_db::{Database, DatabaseOperations};
use scroll_rollup_node_signer::SignatureAsBytes;
use std::{fmt, fmt::Debug, path::PathBuf, sync::Arc};

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
    /// A reference to the rollup-node `Database`.
    rollup_node_db_path: Option<PathBuf>,
    /// The address for which we should persist and serve signatures for.
    signer: Option<Address>,
}

impl ScrollNetworkBuilder {
    /// Create a new [`ScrollNetworkBuilder`] with default configuration.
    pub fn new() -> Self {
        Self {
            scroll_sub_protocols: RlpxSubProtocols::default(),
            rollup_node_db_path: None,
            signer: None,
        }
    }

    /// Add a scroll sub-protocol to the network builder.
    pub fn with_sub_protocol(mut self, protocol: RlpxSubProtocol) -> Self {
        self.scroll_sub_protocols.push(protocol);
        self
    }

    /// Add rollup node database path to the network builder.
    pub fn with_database_path(mut self, db_path: Option<PathBuf>) -> Self {
        self.rollup_node_db_path = db_path;
        self
    }

    /// Set the signer for which we will persist and serve signatures if included in block header
    /// extra data field.
    pub const fn with_signer(mut self, signer: Option<Address>) -> Self {
        self.signer = signer;
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
        // initialize the rollup node database.
        let db_path = ctx.config().datadir().db();
        let database_path = if let Some(database_path) = self.rollup_node_db_path {
            database_path.to_string_lossy().to_string()
        } else {
            // append the path using strings as using `join(...)` overwrites "sqlite://"
            // if the path is absolute.
            let path = db_path.join("scroll.db?mode=rwc");
            "sqlite://".to_string() + &*path.to_string_lossy()
        };
        let db = Arc::new(Database::new(&database_path).await?);

        // get the header transform.
        let chain_spec = ctx.chain_spec();
        let authorized_signer = if self.signer.is_none() {
            match chain_spec.chain().named() {
                Some(NamedChain::Scroll) => Some(SCROLL_MAINNET_SIGNER),
                Some(NamedChain::ScrollSepolia) => Some(SCROLL_SEPOLIA_SIGNER),
                _ => None,
            }
        } else {
            self.signer
        };
        let transform = ScrollHeaderTransform {
            chain_spec: chain_spec.clone(),
            db: db.clone(),
            signer: authorized_signer,
        };
        let request_transform =
            ScrollRequestHeaderTransform { chain_spec, db: db.clone(), signer: authorized_signer };

        // set the network mode to work.
        let config = ctx.network_config()?;
        let config = NetworkConfig {
            network_mode: NetworkMode::Work,
            header_transform: Box::new(transform),
            extra_protocols: self.scroll_sub_protocols,
            ..config
        };

        let network = NetworkManager::builder(config).await?;
        let handle = ctx.start_network(network, pool, Some(Box::new(request_transform)));
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

/// Network primitive types used by Scroll networks.
pub type ScrollNetworkPrimitives =
    BasicNetworkPrimitives<ScrollPrimitives, scroll_alloy_consensus::ScrollPooledTransaction>;

/// The correct signer address for Scroll mainnet and sepolia.
const SCROLL_MAINNET_SIGNER: Address = address!("0xD83C4892BB5aA241B63d8C4C134920111E142A20");
const SCROLL_SEPOLIA_SIGNER: Address = address!("0x687E0E85AD67ff71aC134CF61b65905b58Ab43b2");

/// An implementation of a [`HeaderTransform`] for downloaded headers for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollHeaderTransform<ChainSpec> {
    chain_spec: ChainSpec,
    db: Arc<Database>,
    signer: Option<Address>,
}

impl<ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync + 'static>
    ScrollHeaderTransform<ChainSpec>
{
    /// Returns a new instance of the [`ScrollHeaderTransform`] from the provider chain spec.
    pub const fn new(chain_spec: ChainSpec, db: Arc<Database>, signer: Option<Address>) -> Self {
        Self { chain_spec, db, signer }
    }

    /// Returns a new [`ScrollHeaderTransform`] as a [`HeaderTransform`] trait object.
    pub fn boxed<H: BlockHeader>(
        chain_spec: ChainSpec,
        db: Arc<Database>,
        signer: Option<Address>,
    ) -> Box<dyn HeaderTransform<H>> {
        Box::new(Self { chain_spec, db, signer })
    }
}

impl<H: BlockHeader, ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync>
    HeaderTransform<H> for ScrollHeaderTransform<ChainSpec>
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

impl<ChainSpec: ScrollHardforks + Debug + Send + Sync> ScrollHeaderTransform<ChainSpec> {
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
        persist_signature(self.db.clone(), header.hash_slow(), signature);

        Ok(())
    }
}

/// An implementation of a [`HeaderTransform`] for header request responses for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct ScrollRequestHeaderTransform<ChainSpec> {
    chain_spec: ChainSpec,
    db: Arc<Database>,
    signer: Option<Address>,
}

impl<H: BlockHeader, ChainSpec: EthChainSpec + ScrollHardforks + Debug + Send + Sync>
    HeaderTransform<H> for ScrollRequestHeaderTransform<ChainSpec>
{
    fn map(&self, mut header: H) -> H {
        if !self.chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
            return header;
        }

        // read the signature from the rollup node.
        let signature = tokio::task::block_in_place(|| {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                match handle
                    .block_on(async { self.db.get_block_signature(header.hash_slow()).await })
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
                *header.extra_data_mut() = sig.sig_as_bytes().into();
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
    if authorized_signer.is_some() && Some(signer) != authorized_signer {
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
fn persist_signature(db: Arc<Database>, hash: B256, signature: Signature) {
    tokio::spawn(async move {
        trace!(
            "Persisting block signature to database, block hash: {:?}, sig: {:?}",
            hash,
            signature.to_string()
        );
        if let Err(e) = db.insert_signature(hash, signature).await {
            warn!(target: "scroll::network::header_transform", "Failed to store signature in database: {}", e);
        }
    });
}
