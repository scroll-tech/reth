#[cfg(feature = "test-utils")]
use revm::db::CacheDB;
use revm::{Database, State};

/// Finalize the execution of the type and return the output
pub trait FinalizeExecution<Output> {
    /// Finalize the state and return the output.
    fn finalize(&mut self) -> Output;
}

impl<DB: Database + ContextFul> FinalizeExecution<revm::states::ScrollBundleState> for State<DB> {
    fn finalize(&mut self) -> revm::states::ScrollBundleState {
        let bundle = self.take_bundle();
        (bundle, self.database.context()).into()
    }
}

impl<DB: Database> FinalizeExecution<revm::shared::BundleState> for State<DB> {
    fn finalize(&mut self) -> revm::shared::BundleState {
        self.take_bundle()
    }
}

/// A type that returns additional execution context.
pub trait ContextFul: WithContext<Context = ExecutionContext> {}
impl<T> ContextFul for T where T: WithContext<Context = ExecutionContext> {}

/// Types that can provide a context.
#[auto_impl::auto_impl(&, &mut)]
pub trait WithContext {
    /// The context returned.
    type Context;

    /// Returns the context from the type.
    fn context(&self) -> &Self::Context;
}

/// The default empty post execution context.
#[cfg(not(feature = "scroll"))]
pub type ExecutionContext = ();
/// The Scroll execution context hidden behind a feature flag.
#[cfg(feature = "scroll")]
pub type ExecutionContext = reth_scroll_primitives::ScrollPostExecutionContext;

/// A default static post execution context.
#[cfg(any(not(feature = "scroll"), feature = "test-utils"))]
pub static DEFAULT_EMPTY_CONTEXT: std::sync::LazyLock<ExecutionContext> =
    std::sync::LazyLock::new(Default::default);

#[cfg(feature = "test-utils")]
impl<DB> WithContext for CacheDB<DB> {
    type Context = ExecutionContext;

    fn context(&self) -> &Self::Context {
        &DEFAULT_EMPTY_CONTEXT
    }
}
