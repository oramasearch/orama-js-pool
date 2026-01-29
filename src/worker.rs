use std::collections::HashMap;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::warn;

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

    pub fn builder() -> WorkerBuilder {
        WorkerBuilder::default()
    }

    /// Add a module to this worker
    pub async fn add_module<Code>(&mut self, name: String, code: Code) -> Result<(), RuntimeError>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        let code_string: ModuleCodeString = code.into();
        let (runtime_code, module_code) = code_string.into_cheap_copy();

        let runtime = self.get_runtime().await?;
        runtime.load_module(name.clone(), runtime_code).await?;

        self.modules.insert(
            name.clone(),
            ModuleInfo {
                code: module_code.as_str().into(),
            },
        );

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
        if !self.modules.contains_key(module_name) {
            return Err(RuntimeError::MissingModule(module_name.to_string()));
        }

        let domain_permission = exec_options
            .domain_permission
            .unwrap_or_else(|| self.domain_permission.clone());

        let runtime = self.get_runtime().await?;

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
                domain_permission,
                exec_options.timeout,
            )
            .await?;

        let output: Output = serde_json::from_value(result)?;
        Ok(output)
    }

    // Checks if the runtime is healthy otherwise it recreate it.
    async fn get_runtime(
        &mut self,
    ) -> Result<&mut Runtime<serde_json::Value, serde_json::Value>, RuntimeError> {
        let needs_rebuild = !matches!(&self.runtime, Some(rt) if rt.is_alive());

        if needs_rebuild {
            warn!("Runtime not alive or missing, rebuilding...");
            self.rebuild_runtime().await?;
        }

        Ok(self.runtime.as_mut().unwrap())
    }

    /// Rebuild the runtime with all currently registered modules
    async fn rebuild_runtime(&mut self) -> Result<(), RuntimeError> {
        let mut runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
            self.domain_permission.clone(),
            self.evaluation_timeout,
            self.cache.clone(),
        )
        .await?;

        for (name, info) in &self.modules {
            runtime.load_module(name.clone(), info.code.clone()).await?;
        }

        self.runtime = Some(runtime);

        Ok(())
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
        let domain_permission = self.domain_permission.unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_module_evaluation_timeout() {
        let _ = tracing_subscriber::fmt::try_init();

        let expensive_code = r#"
            await new Promise(resolve => setTimeout(resolve, 10000));
            function getValue() {
                return 42;
            }
            export default { getValue };
        "#;

        let not_expensive_code = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let result = Worker::builder()
            .with_evaluation_timeout(Duration::from_millis(100))
            .add_module("expensive", expensive_code.to_string())
            .build()
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        assert!(matches!(result, Err(RuntimeError::InitTimeout)));

        let mut worker = Worker::builder()
            .with_evaluation_timeout(Duration::from_millis(100))
            .add_module("not_expensive", not_expensive_code.to_string())
            .build()
            .await
            .unwrap();

        // also on adding a module
        let result = worker
            .add_module("expensive".into(), expensive_code.to_string())
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        assert!(matches!(result, Err(RuntimeError::InitTimeout)));

        worker
            .add_module("not_expensive_2".into(), not_expensive_code.to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_module_evaluation_domain_permission() {
        let _ = tracing_subscriber::fmt::try_init();

        let not_allowed_code = r#"
            let res = await fetch("http://foo.test");
            let value = await res.text();

            function getValue() {
                return value;
            }
            export default { getValue };
        "#;

        let result = Worker::builder()
            .with_domain_permission(DomainPermission::DenyAll)
            .add_module("net_call", not_allowed_code.to_string())
            .build()
            .await;

        assert!(
            result.is_err(),
            "Module evaluation should fail due to network deny"
        );
        match result {
            Err(RuntimeError::InitializationError(e)) => {
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("network access is denied"),
                    "Error should contain network deny information, got: {error_msg}",
                );
            }
            _ => panic!("Expected InitializationError with network deny information"),
        }
    }

    #[tokio::test]
    async fn test_module_override() {
        let _ = tracing_subscriber::fmt::try_init();

        let original_code = r#"
            function getValue() { return 42; }
            export default { getValue };
        "#;

        let override_code = r#"
            function getValue() { return 100; }
            export default { getValue };
        "#;

        let mut worker = Worker::builder()
            .add_module("test", original_code.to_string())
            .build()
            .await
            .unwrap();

        let result: i32 = worker
            .exec("test", "getValue", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result, 42);

        // Override the module
        worker
            .add_module("test".into(), override_code.to_string())
            .await
            .unwrap();

        let result: i32 = worker
            .exec("test", "getValue", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result, 100, "Function should return overridden value");
    }
}
