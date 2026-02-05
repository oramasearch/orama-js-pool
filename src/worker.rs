use std::collections::HashMap;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::orama_extension::SharedCache;
use crate::runtime::ModuleName;

use super::{
    options::{DomainPermission, ExecOptions, MaxExecutions},
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
    runtime: Option<Runtime>,
    modules: HashMap<String, ModuleInfo>,
    cache: SharedCache,
    domain_permission: DomainPermission,
    evaluation_timeout: std::time::Duration,
    max_executions: MaxExecutions,
    execution_count: u64,
    version: u64,
}

impl Worker {
    /// Create a new worker with the given cache, and settings
    pub(crate) fn new(
        cache: SharedCache,
        domain_permission: DomainPermission,
        evaluation_timeout: std::time::Duration,
        max_executions: MaxExecutions,
        version: u64,
    ) -> Self {
        Self {
            runtime: None,
            modules: HashMap::new(),
            cache,
            domain_permission,
            evaluation_timeout,
            max_executions,
            execution_count: 0,
            version,
        }
    }

    /// Get the version of this worker
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn builder() -> WorkerBuilder {
        WorkerBuilder::default()
    }

    /// Add a module to this worker
    pub async fn add_module<Code>(
        &mut self,
        name: impl Into<String>,
        code: Code,
    ) -> Result<(), RuntimeError>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        let name_string = name.into();
        let validated_name = ModuleName::new(&name_string)?;
        let code_string: ModuleCodeString = code.into();
        let (runtime_code, module_code) = code_string.into_cheap_copy();

        let runtime = self.get_runtime().await?;
        runtime.load_module(validated_name, runtime_code).await?;

        self.modules.insert(
            name_string,
            ModuleInfo {
                code: module_code.as_str().into(),
            },
        );

        Ok(())
    }

    /// Remove a module from this worker and recreate the runtime
    /// This helps free memory by recreating the runtime without the removed module
    pub async fn remove_module(&mut self, name: &str) -> Result<(), RuntimeError> {
        if !self.modules.contains_key(name) {
            return Err(RuntimeError::MissingModule(name.to_string()));
        }

        self.modules.remove(name);

        // Rebuild the runtime to recreate it without the new module
        self.rebuild_runtime().await?;

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

        // Check if we need to invalidate the runtime due to execution limit
        if self.max_executions.is_exceeded(self.execution_count) {
            warn!("Worker reached max executions limit, invalidating runtime");
            self.runtime = None;
            self.execution_count = 0;
        }

        let domain_permission = exec_options
            .domain_permission
            .unwrap_or_else(|| self.domain_permission.clone());

        let runtime = self.get_runtime().await?;

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

        self.execution_count += 1;

        let output: Output = serde_json::from_value(result)?;
        Ok(output)
    }

    // Checks if the runtime is healthy otherwise it recreate it.
    async fn get_runtime(&mut self) -> Result<&mut Runtime, RuntimeError> {
        let needs_rebuild = !matches!(&self.runtime, Some(rt) if rt.is_alive());

        if needs_rebuild {
            warn!("Runtime not alive or missing, rebuilding...");
            self.rebuild_runtime().await?;
        }

        self.runtime.as_mut().ok_or(RuntimeError::Terminated)
    }

    /// Rebuild the runtime with all currently registered modules
    async fn rebuild_runtime(&mut self) -> Result<(), RuntimeError> {
        let mut runtime = Runtime::new(
            self.domain_permission.clone(),
            self.evaluation_timeout,
            self.cache.clone(),
        )
        .await?;

        for (name, info) in &self.modules {
            let validated_name = ModuleName::new(name.clone())?;
            runtime
                .load_module(validated_name, info.code.clone())
                .await?;
        }

        self.runtime = Some(runtime);

        Ok(())
    }

    /// Check if the worker is alive
    pub fn is_alive(&self) -> bool {
        self.runtime.as_ref().is_some_and(|rt| rt.is_alive())
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
    domain_permission: Option<DomainPermission>,
    evaluation_timeout: Option<std::time::Duration>,
    max_executions: MaxExecutions,
    version: u64,
}

impl WorkerBuilder {
    /// Create a new WorkerBuilder
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            cache: None,
            domain_permission: None,
            evaluation_timeout: None,
            max_executions: MaxExecutions::default(),
            version: 0,
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

    /// Set the maximum number of executions before recycling the runtime.
    pub fn with_max_executions(mut self, max: MaxExecutions) -> Self {
        self.max_executions = max;
        self
    }

    /// Set the version for the worker
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }

    /// Build the worker
    pub async fn build(self) -> Result<Worker, RuntimeError> {
        let cache = self.cache.unwrap_or_default();
        let domain_permission = self.domain_permission.unwrap_or_default();
        let evaluation_timeout = self
            .evaluation_timeout
            .unwrap_or(std::time::Duration::from_secs(5));

        let mut worker = Worker::new(
            cache,
            domain_permission,
            evaluation_timeout,
            self.max_executions,
            self.version,
        );

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
            .add_module("expensive", expensive_code.to_string())
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        assert!(matches!(result, Err(RuntimeError::InitTimeout)));

        worker
            .add_module("not_expensive_2", not_expensive_code.to_string())
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
            .add_module("test", override_code.to_string())
            .await
            .unwrap();

        let result: i32 = worker
            .exec("test", "getValue", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result, 100, "Function should return overridden value");
    }

    #[tokio::test]
    async fn test_max_executions() {
        let _ = tracing_subscriber::fmt::try_init();

        let counter_code = r#"
            let callCount = 0;
            function increment() {
                callCount++;
                return callCount;
            }
            export default { increment };
        "#;

        let mut worker = Worker::builder()
            .add_module("counter", counter_code.to_string())
            .with_max_executions(MaxExecutions::Limited(3))
            .build()
            .await
            .unwrap();

        // First 3 executions should increment the counter
        let result1: i32 = worker
            .exec("counter", "increment", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result1, 1);

        let result2: i32 = worker
            .exec("counter", "increment", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result2, 2);

        let result3: i32 = worker
            .exec("counter", "increment", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result3, 3);

        // After 3 executions, the runtime should be recycled
        // and the counter should reset to 1
        let result4: i32 = worker
            .exec("counter", "increment", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(
            result4, 1,
            "Counter should reset after max_executions is reached"
        );
    }

    #[tokio::test]
    async fn test_runtime_error_on_invalid_function_code() {
        let _ = tracing_subscriber::fmt::try_init();

        // This code has a syntax error in the function body that will only be
        // triggered when the function is executed (not during module evaluation)
        let code_with_runtime_syntax_error = r#"
            function badFunction() {
                // This will cause a syntax error during dynamic code generation
                eval('this is not valid javascript!!!');
                return 42;
            }
            export default { badFunction };
        "#;

        let mut worker = Worker::builder()
            .add_module("test", code_with_runtime_syntax_error.to_string())
            .build()
            .await
            .unwrap();

        let result: Result<i32, RuntimeError> = worker
            .exec("test", "badFunction", (), ExecOptions::default())
            .await;

        // Should return an error, not panic
        assert!(
            result.is_err(),
            "Should return error for runtime syntax error"
        );
        assert!(
            matches!(result.unwrap_err(), RuntimeError::ErrorThrown(_)),
            "Should return ErrorThrown variant"
        );
    }

    #[tokio::test]
    async fn test_runtime_error_on_eval_failure() {
        let _ = tracing_subscriber::fmt::try_init();

        // This code will fail during the eval phase (after load_side_es_module_from_code)
        // by throwing an error during async function execution
        let code_with_async_error = r#"
            async function throwingFunction() {
                // Force an error during execution that affects the eval phase
                await Promise.reject(new Error("Async execution failed"));
                return 42;
            }
            export default { throwingFunction };
        "#;

        let mut worker = Worker::builder()
            .add_module("test", code_with_async_error.to_string())
            .build()
            .await
            .unwrap();

        let result: Result<i32, RuntimeError> = worker
            .exec("test", "throwingFunction", (), ExecOptions::default())
            .await;

        // Should return an error, not panic
        assert!(
            result.is_err(),
            "Should return error for async execution failure"
        );
        assert!(
            matches!(result.unwrap_err(), RuntimeError::ErrorThrown(_)),
            "Should return ErrorThrown variant"
        );
    }

    #[tokio::test]
    async fn test_invalid_module_name() {
        let _ = tracing_subscriber::fmt::try_init();

        // Test that invalid module names are caught early
        let code = r#"
            function test() { return 42; }
            export default { test };
        "#;

        let mut worker = Worker::builder().build().await.unwrap();

        // Empty module name should fail
        let result = worker.add_module("", code.to_string()).await;
        assert!(result.is_err(), "Empty module name should fail");
        assert!(
            matches!(result.unwrap_err(), RuntimeError::InvalidModuleName(_, _)),
            "Should return InvalidModuleName error"
        );
    }

    #[tokio::test]
    async fn test_remove_module() {
        let _ = tracing_subscriber::fmt::try_init();

        let code1 = r#"
            function getValue() { return 42; }
            export default { getValue };
        "#;

        let code2 = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let mut worker = Worker::builder()
            .add_module("module1", code1.to_string())
            .add_module("module2", code2.to_string())
            .build()
            .await
            .unwrap();

        let result1: i32 = worker
            .exec("module1", "getValue", (), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result1, 42);

        let result2: i32 = worker
            .exec("module2", "add", (5, 3), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result2, 8);

        worker.remove_module("module1").await.unwrap();

        let result: Result<i32, RuntimeError> = worker
            .exec("module1", "getValue", (), ExecOptions::default())
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeError::MissingModule(name) if name == "module1"
        ));

        let result2: i32 = worker
            .exec("module2", "add", (10, 5), ExecOptions::default())
            .await
            .unwrap();
        assert_eq!(result2, 15);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_module() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut worker = Worker::builder().build().await.unwrap();

        // Removing a module that doesn't exist should fail
        let result = worker.remove_module("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeError::MissingModule(name) if name == "nonexistent"
        ));
    }
}
