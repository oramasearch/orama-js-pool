use std::sync::Arc;
use std::time::Duration;

use crate::OutputChannel;

pub use crate::permission::DomainPermission;

/// Controls how many times a worker can execute functions before being recycled.
///
/// Worker recycling helps prevent memory leaks from accumulated module cache and state.
/// When a worker reaches its execution limit, it will be invalidated and recreated
/// with a fresh runtime on the next use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxExecutions {
    Unlimited,
    Limited(u64),
}

impl Default for MaxExecutions {
    fn default() -> Self {
        MaxExecutions::Limited(100)
    }
}

impl MaxExecutions {
    /// Check if the execution count has been exceeded
    pub fn is_exceeded(&self, count: u64) -> bool {
        match self {
            MaxExecutions::Unlimited => false,
            MaxExecutions::Limited(max) => count >= *max,
        }
    }
}

/// Policy for when to recycle/invalidate a runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecyclePolicy {
    #[default]
    Never,
    /// Recycle on timeout only (current default behavior)
    OnTimeout,
    /// Recycle on any error
    OnError,
    /// Recycle on timeout or error
    OnTimeoutOrError,
}

#[derive(Debug, Clone, Default)]
pub struct ExecOptions {
    pub timeout: Option<Duration>,
    pub domain_permission: Option<DomainPermission>,
    pub stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
}

impl ExecOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = Some(permission);
        self
    }

    pub fn with_stdout_sender(
        mut self,
        stdout_sender: Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>,
    ) -> Self {
        self.stdout_sender = Some(stdout_sender);
        self
    }
}

/// Worker-level options that apply to all modules in a worker
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    pub evaluation_timeout: Duration,
    pub execution_timeout: Duration,
    pub domain_permission: DomainPermission,
    pub recycle_policy: RecyclePolicy,
    pub max_executions: MaxExecutions,
}

impl WorkerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_evaluation_timeout(mut self, timeout: Duration) -> Self {
        self.evaluation_timeout = timeout;
        self
    }

    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = timeout;
        self
    }

    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = permission;
        self
    }

    pub fn with_recycle_policy(mut self, policy: RecyclePolicy) -> Self {
        self.recycle_policy = policy;
        self
    }

    pub fn with_max_executions(mut self, max: MaxExecutions) -> Self {
        self.max_executions = max;
        self
    }
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            evaluation_timeout: Duration::from_secs(5),
            execution_timeout: Duration::from_secs(30),
            domain_permission: DomainPermission::DenyAll,
            recycle_policy: RecyclePolicy::default(),
            max_executions: MaxExecutions::default(),
        }
    }
}
