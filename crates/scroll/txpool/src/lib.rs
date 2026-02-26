//! Transaction pool for Scroll node.

mod transaction;
pub use transaction::ScrollPooledTransaction;

mod validator;
pub use validator::{ScrollL1BlockInfo, ScrollTransactionValidator};

use reth_scroll_evm::ScrollEvmConfig;
use reth_transaction_pool::{CoinbaseTipOrdering, Pool, TransactionValidationTaskExecutor};

/// Type alias for default scroll transaction pool
pub type ScrollTransactionPool<Client, S, T = ScrollPooledTransaction, Evm = ScrollEvmConfig> =
    Pool<
        TransactionValidationTaskExecutor<ScrollTransactionValidator<Client, T, Evm>>,
        CoinbaseTipOrdering<T>,
        S,
    >;
