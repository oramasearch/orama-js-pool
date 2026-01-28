#![doc = include_str!("../README.md")]

mod executor;
pub mod new;
mod orama_extension;
mod parameters;
mod permission;
mod pool;
mod runner;

// Re-export new API
pub use new::{
    DomainPermission, ExecOptions, ModuleDefinition, ModuleOptions, Pool, PoolBuilder,
    ResolvedExecOptions, Runtime, RuntimeError, Worker, WorkerBuilder, WorkerManager,
};

// Re-export common types
pub use orama_extension::{OutputChannel, SharedCache};
pub use parameters::*;

// Old API (deprecated - will be removed in future version)
#[deprecated(
    since = "0.3.0",
    note = "Use the new Pool API instead. See migration guide in documentation."
)]
pub use executor::*;

#[deprecated(
    since = "0.3.0",
    note = "Use the new Pool API instead. See migration guide in documentation."
)]
pub use pool::{JSPoolExecutor, JSPoolExecutorConfig};

#[deprecated(since = "0.3.0", note = "Use RuntimeError instead")]
pub use runner::{ExecOption, JSRunnerError};
