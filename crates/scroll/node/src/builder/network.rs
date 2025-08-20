use alloy_primitives::{address, Address, Signature};
use scroll_rollup_node_db::Database;
use std::fmt;
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
use reth_tracing::tracing::info;
use reth_transaction_pool::{PoolTransaction, TransactionPool};
use scroll_alloy_hardforks::ScrollHardforks;
use std::{fmt::Debug, sync::Arc};

/// Errors that can occur during signature validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureError {
    /// Invalid signature length (expected 65 bytes)
    InvalidSignature,
    /// Invalid signer (not authorized)
    InvalidSigner,
    /// Signature recovery failed
    RecoveryFailed,
    /// Database operation failed
    DatabaseError(String),
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureError::InvalidSignature => write!(f, "Invalid signature length, expected 65 bytes"),
            SignatureError::InvalidSigner => write!(f, "Invalid signer, not authorized"),
            SignatureError::RecoveryFailed => write!(f, "Failed to recover signer from signature"),
            SignatureError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for SignatureError {}

/// The network builder for Scroll.
#[derive(Debug, Default)]
pub struct ScrollNetworkBuilder {
    /// Additional `RLPx` sub-protocols to be added to the network.
    scroll_sub_protocols: RlpxSubProtocols,
    /// A reference to the rollup-node `Database`.
    rollup_node_db: Arc<Database>,
}

impl ScrollNetworkBuilder {
    /// Create a new [`ScrollNetworkBuilder`] with default configuration.
    pub fn new(database: Arc<Database>) -> Self {
        Self { scroll_sub_protocols: RlpxSubProtocols::default(), rollup_node_db: database }
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
        // get the header transform.
        let chain_spec = ctx.chain_spec();
        let transform = ScrollHeaderTransform { chain_spec, db: self.rollup_node_db };

        // set the network mode to work.
        let config = ctx.network_config()?;
        let config = NetworkConfig {
            network_mode: NetworkMode::Work,
            header_transform: Box::new(transform),
            extra_protocols: self.scroll_sub_protocols,
            ..config
        };

        let network = NetworkManager::builder(config).await?;
        let handle = ctx.start_network(network, pool);
        info!(target: "reth::cli", enode=%handle.local_node_record(), "P2P networking initialized");
        Ok(handle)
    }
}

/// Network primitive types used by Scroll networks.
pub type ScrollNetworkPrimitives =
    BasicNetworkPrimitives<ScrollPrimitives, scroll_alloy_consensus::ScrollPooledTransaction>;

/// The correct signer address for Scroll mainnet.
const SCROLL_MAINNET_SIGNER: Address = address!("0000000000000000000000000000000000000000");
const SCROLL_SEPOLIA_SIGNER: Address = address!("0000000000000000000000000000000000000000");

/// An implementation of a [`HeaderTransform`] for Scroll.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ScrollHeaderTransform<ChainSpec> {
    chain_spec: ChainSpec,
    db: Arc<Database>,
}

impl<ChainSpec: ScrollHardforks + Debug + Send + Sync + 'static> ScrollHeaderTransform<ChainSpec> {
    /// Returns a new instance of the [`ScrollHeaderTransform`] from the provider chain spec.
    pub const fn new(chain_spec: ChainSpec, db: Arc<Database>) -> Self {
        Self { chain_spec, db }
    }

    /// Returns a new [`ScrollHeaderTransform`] as a [`HeaderTransform`] trait object.
    pub fn boxed<H: BlockHeader>(chain_spec: ChainSpec, db: Arc<Database>) -> Box<dyn HeaderTransform<H>> {
        Box::new(Self { chain_spec, db })
    }
}

impl<H: BlockHeader, ChainSpec: ScrollHardforks + Debug + Send + Sync> HeaderTransform<H>
    for ScrollHeaderTransform<ChainSpec>
{
    fn map(&self, mut header: H) -> H {
        if self.chain_spec.is_euclid_v2_active_at_timestamp(header.timestamp()) {
            // clear the extra data field.
            // *header.extra_data_mut() = Default::default()

            // TODO: remove this once we deprecated l2geth
            // Validate and process signature
            if let Err(err) = self.validate_signature(&mut header) {
                reth_tracing::tracing::warn!("Header signature validation failed, header hash: {:?}, error: {}", header.hash_slow(), err);
                return H::default();
            }
        }
        header
    }
}

impl<ChainSpec: ScrollHardforks + Debug + Send + Sync> ScrollHeaderTransform<ChainSpec>
{
    fn validate_signature<H: BlockHeader>(&self, header: &mut H) -> Result<(), SignatureError> {
        let signature_bytes = std::mem::take(header.extra_data_mut());
        
        // Parse 65-byte signature: [r (32 bytes), s (32 bytes), v (1 byte)]
        if signature_bytes.len() != 65 {
            return Err(SignatureError::InvalidSignature);
        }
        
        let r = alloy_primitives::U256::from_be_slice(&signature_bytes[0..32]);
        let s = alloy_primitives::U256::from_be_slice(&signature_bytes[32..64]);
        let v = signature_bytes[64];
        let parity = v != 0;
        
        let signature = Signature::new(r, s, parity);

        // Recover signer from signature
        let signer = reth_primitives_traits::crypto::secp256k1::recover_signer(&signature, header.hash_slow())
            .map_err(|_| SignatureError::RecoveryFailed)?;
            
        // Verify signer is authorized
        if SCROLL_MAINNET_SIGNER != signer {
            return Err(SignatureError::InvalidSigner);
        }
        
        // Store signature in database
        self.db.insert_signature(header.hash_slow(), signature)
            .map_err(|e| SignatureError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
}