use std::collections::HashMap;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::orama_extension::SharedCache;

use super::{
    options::{DomainPermission, ExecOptions},
    parameters::TryIntoFunctionParameters,
    runtime::{Runtime, RuntimeError},
};

use std::sync::Arc;

/// Metadata about a loaded module
struct ModuleInfo {
    code: Arc<str>,
}

/// Worker that can execute multiple modules with a shared runtime
pub struct Worker {
    runtime: Option<Runtime<serde_json::Value, serde_json::Value>>,
    modules: HashMap<String, ModuleInfo>,
    cache: SharedCache,
    version: u64,
    domain_permission: DomainPermission,
    evaluation_timeout: std::time::Duration,
}

impl Worker {
    /// Create a new worker with the given cache, version, and settings
    pub(crate) fn new(
        cache: SharedCache,
        version: u64,
        domain_permission: DomainPermission,
        evaluation_timeout: std::time::Duration,
    ) -> Self {
        Self {
            runtime: None,
            modules: HashMap::new(),
            cache,
            version,
            domain_permission,
            evaluation_timeout,
        }
    }

    pub fn build() -> WorkerBuilder {
        WorkerBuilder::default()
    }

    /// Add a module to this worker
    pub async fn add_module<Code>(&mut self, name: String, code: Code) -> Result<(), RuntimeError>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        let code_string: ModuleCodeString = code.into();
        let specifier = format!("file:/{name}");

        info!("Adding module: {} with specifier {}", name, specifier);

        // Store module info
        self.modules.insert(
            name.clone(),
            ModuleInfo {
                code: code_string.as_str().into(),
            },
        );

        // Create runtime if it doesn't exist
        if self.runtime.is_none() {
            info!("Creating new runtime");
            let runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
                self.domain_permission.clone(),
                self.evaluation_timeout,
                self.cache.clone(),
            )
            .await?;
            self.runtime = Some(runtime);
        }

        // Load this module into the runtime
        if let Some(runtime) = &mut self.runtime {
            runtime.load_module(name.clone(), code_string).await?;
        }

        Ok(())
    }

    /// Rebuild the runtime with all currently registered modules
    async fn rebuild_runtime(&mut self) -> Result<(), RuntimeError> {
        info!("Rebuilding runtime with {} modules", self.modules.len());

        // Create a new runtime
        let mut runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
            self.domain_permission.clone(),
            self.evaluation_timeout,
            self.cache.clone(),
        )
        .await?;

        // Load all modules into the new runtime
        for (name, info) in &self.modules {
            runtime.load_module(name.clone(), info.code.clone()).await?;
        }

        self.runtime = Some(runtime);

        Ok(())
    }

    /// Execute a function in a module
    pub async fn exec<Input, Output>(
        &mut self,
        module_name: &str,
        function_name: &str,
        params: Input,
        exec_options: ExecOptions,
    ) -> Result<Output, RuntimeError>
    where
        Input: TryIntoFunctionParameters + Send + 'static,
        Output: DeserializeOwned + Send + 'static,
    {
        // Check if module exists
        if !self.modules.contains_key(module_name) {
            return Err(RuntimeError::MissingModule(module_name.to_string()));
        }

        // Check if runtime is alive, recreate if needed
        let runtime = match &mut self.runtime {
            Some(rt) if rt.is_alive() => rt,
            _ => {
                info!("Runtime not alive or missing, rebuilding...");
                self.rebuild_runtime().await?;
                self.runtime.as_mut().unwrap()
            }
        };

        // Check if the function exists in the module
        runtime
            .check_function(module_name, function_name.to_string())
            .await?;

        let params_tuple = params.try_into_function_parameter()?;
        let params_value = serde_json::to_value(params_tuple.0)?;

        let result: serde_json::Value = runtime
            .exec(
                module_name,
                function_name.to_string(),
                params_value,
                exec_options.stdout_sender,
                exec_options.domain_permission,
                exec_options.timeout,
            )
            .await?;

        // Convert output from serde_json::Value
        let output: Output = serde_json::from_value(result)?;
        Ok(output)
    }

    /// Check if the worker is alive
    pub fn is_alive(&self) -> bool {
        self.runtime.as_ref().is_some_and(|rt| rt.is_alive())
    }

    /// Get the version of this worker
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the shared cache
    pub fn cache(&self) -> &SharedCache {
        &self.cache
    }
}

/// Builder for creating a Worker
pub struct WorkerBuilder {
    modules: Vec<(String, ModuleCodeString)>,
    cache: Option<SharedCache>,
    version: Option<u64>,
    domain_permission: Option<DomainPermission>,
    evaluation_timeout: Option<std::time::Duration>,
}

impl WorkerBuilder {
    /// Create a new WorkerBuilder
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            cache: None,
            version: None,
            domain_permission: None,
            evaluation_timeout: None,
        }
    }

    /// Add a module to the worker
    pub fn add_module<Code: Into<ModuleCodeString>>(
        mut self,
        name: impl Into<String>,
        code: Code,
    ) -> Self {
        let code: ModuleCodeString = code.into();
        self.modules.push((name.into(), code));
        self
    }

    /// Set the cache for the worker
    pub fn with_cache(mut self, cache: SharedCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set the version for the worker
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }

    /// Set the allowed hosts for all modules
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.domain_permission = Some(DomainPermission::Allow(hosts));
        self
    }

    /// Set the domain permission for all modules
    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = Some(permission);
        self
    }

    /// Set the evaluation timeout for module loading
    pub fn with_evaluation_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.evaluation_timeout = Some(timeout);
        self
    }

    /// Build the worker
    pub async fn build(self) -> Result<Worker, RuntimeError> {
        let cache = self.cache.unwrap_or_default();
        let version = self.version.unwrap_or(0);
        let domain_permission = self.domain_permission.unwrap_or(DomainPermission::DenyAll);
        let evaluation_timeout = self
            .evaluation_timeout
            .unwrap_or(std::time::Duration::from_secs(5));

        let mut worker = Worker::new(cache, version, domain_permission, evaluation_timeout);

        for (name, code) in self.modules {
            worker.add_module(name, code).await?;
        }

        Ok(worker)
    }
}

impl Default for WorkerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
