use std::sync::Arc;
use std::time::Duration;

use crate::OutputChannel;

pub use crate::permission::DomainPermission;

/// Policy for when to recycle/invalidate a runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecyclePolicy {
    /// Recycle on timeout only (current default behavior)
    OnTimeout,
    /// Recycle on any error
    OnError,
    /// Recycle on timeout or error
    #[default]
    OnTimeoutOrError,
}

#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub timeout: Duration,
    pub domain_permission: Option<DomainPermission>,
    pub stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
}

impl ExecOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
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

impl Default for ExecOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            domain_permission: None,
            stdout_sender: None,
        }
    }
}

/// Worker-level options that apply to all modules in a worker
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    pub evaluation_timeout: Duration,
    pub domain_permission: DomainPermission,
    pub recycle_policy: RecyclePolicy,
}

impl WorkerOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_evaluation_timeout(mut self, timeout: Duration) -> Self {
        self.evaluation_timeout = timeout;
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
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            evaluation_timeout: Duration::from_secs(5),
            domain_permission: DomainPermission::DenyAll,
            recycle_policy: RecyclePolicy::default(),
        }
    }
}
