use std::sync::Arc;
use std::time::Duration;

use crate::OutputChannel;

#[derive(Debug, Clone)]
pub enum DomainPermission {
    // TODO: add denyAll, AllowAll, Deny(Vec<String>)
    Deny,
    Allow(Vec<String>),
}

impl DomainPermission {
    pub fn to_allowed_hosts(&self) -> Option<Vec<String>> {
        match self {
            DomainPermission::Deny => Some(vec![]),
            DomainPermission::Allow(hosts) => Some(hosts.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleOptions {
    pub timeout: Duration,
    pub domain_permission: DomainPermission,
}

impl ModuleOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = permission;
        self
    }
}

impl Default for ModuleOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            domain_permission: DomainPermission::Deny,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub timeout: Duration,
    pub domain_permission: DomainPermission,
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
        self.domain_permission = permission;
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
            domain_permission: DomainPermission::Deny,
            stdout_sender: None,
        }
    }
}

/// Worker-level options that apply to all modules in a worker
#[derive(Debug, Clone)]
pub struct WorkerOptions {
    pub evaluation_timeout: Duration,
    pub domain_permission: DomainPermission,
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
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            evaluation_timeout: Duration::from_secs(5),
            domain_permission: DomainPermission::Deny,
        }
    }
}

/// Resolved execution options after merging module and runtime options
#[derive(Debug, Clone)]
pub struct ResolvedExecOptions {
    pub timeout: Duration,
    pub allowed_hosts: Option<Vec<String>>,
}
