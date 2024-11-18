use reth_revm::{database::StateProviderDatabase, db::CacheDB, Database, State};
use std::sync::LazyLock;

/// Finalize the execution of the type and return the output
pub trait FinalizeExecution<Output> {
    /// Finalize the state and return the output.
    fn finalize(&mut self) -> Output;
}

impl<DB: Database + ContextFul> FinalizeExecution<reth_scroll_revm::states::ScrollBundleState>
    for State<DB>
{
    fn finalize(&mut self) -> reth_scroll_revm::states::ScrollBundleState {
        let bundle = self.take_bundle();
        (bundle, self.database.context()).into()
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

#[cfg(not(feature = "scroll"))]
type ExecutionContext = ();
#[cfg(feature = "scroll")]
type ExecutionContext = reth_scroll_primitives::ScrollPostExecutionContext;
static DEFAULT_CONTEXT: LazyLock<ExecutionContext> = LazyLock::new(Default::default);

#[cfg(feature = "scroll")]
impl<DB> WithContext for reth_scroll_storage::ScrollStateProviderDatabase<DB> {
    type Context = ExecutionContext;

    fn context(&self) -> &Self::Context {
        &self.post_execution_context
    }
}

impl<DB> WithContext for CacheDB<DB> {
    type Context = ExecutionContext;

    fn context(&self) -> &Self::Context {
        &DEFAULT_CONTEXT
    }
}

impl<DB> WithContext for StateProviderDatabase<DB> {
    type Context = ExecutionContext;

    fn context(&self) -> &Self::Context {
        &DEFAULT_CONTEXT
    }
}
