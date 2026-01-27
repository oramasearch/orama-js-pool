use std::time::Duration;

/// Permission settings for domain/network access
#[derive(Debug, Clone)]
pub enum DomainPermission {
    /// Deny all network access
    Deny,
    /// Allow access to specific domains
    Allow(Vec<String>),
}

impl DomainPermission {
    /// Convert to Option<Vec<String>> for compatibility with existing permission system
    pub fn to_allowed_hosts(&self) -> Option<Vec<String>> {
        match self {
            DomainPermission::Deny => Some(vec![]),
            DomainPermission::Allow(hosts) => Some(hosts.clone()),
        }
    }
}

/// Options for module initialization
#[derive(Debug, Clone)]
pub struct ModuleOptions {
    /// Timeout for module initialization
    pub timeout: Duration,
    /// Domain/network permissions during module initialization
    pub domain_permission: DomainPermission,
}

impl Default for ModuleOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            domain_permission: DomainPermission::Deny,
        }
    }
}

/// Options for function execution (runtime overrides)
#[derive(Debug, Clone, Default)]
pub struct ExecOptions {
    /// Override module timeout for this execution
    pub timeout: Option<Duration>,
    /// Override module domain permissions for this execution
    pub domain_permission: Option<DomainPermission>,
}

impl ExecOptions {
    /// Create new ExecOptions with no overrides
    pub fn new() -> Self {
        Self::default()
    }

    /// Set timeout override
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set domain permission override
    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = Some(permission);
        self
    }

    /// Merge with module options - runtime overrides take precedence
    pub fn merge_with_module(&self, module_opts: &ModuleOptions) -> ResolvedExecOptions {
        ResolvedExecOptions {
            timeout: self.timeout.unwrap_or(module_opts.timeout),
            allowed_hosts: self
                .domain_permission
                .as_ref()
                .unwrap_or(&module_opts.domain_permission)
                .to_allowed_hosts(),
        }
    }
}

/// Resolved execution options after merging module and runtime options
#[derive(Debug, Clone)]
pub struct ResolvedExecOptions {
    pub timeout: Duration,
    pub allowed_hosts: Option<Vec<String>>,
}
