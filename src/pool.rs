use std::collections::HashMap;
use std::time::Duration;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::orama_extension::SharedCache;

use super::{
    manager::{ModuleDefinition, WorkerManager},
    options::{DomainPermission, ExecOptions, WorkerOptions},
    parameters::TryIntoFunctionParameters,
    runtime::RuntimeError,
};

pub struct Pool {
    inner: deadpool::managed::Pool<WorkerManager>,
    manager: WorkerManager,
}

impl Pool {
    pub fn builder() -> PoolBuilder {
        PoolBuilder::new()
    }

    // Execute a function in a module
    pub async fn exec<Input, Output>(
        &self,
        module_name: &str,
        function_name: &str,
        params: Input,
        exec_options: ExecOptions,
    ) -> Result<Output, RuntimeError>
    where
        Input: TryIntoFunctionParameters + Send + 'static,
        Output: DeserializeOwned + Send + 'static,
    {
        let mut worker = self
            .inner
            .get()
            .await
            .map_err(|e| RuntimeError::Unknown(e.to_string()))?;

        worker
            .exec(module_name, function_name, params, exec_options)
            .await
    }

    /// Add or update a module in the pool
    pub async fn add_module<Code: Into<ModuleCodeString>>(
        &self,
        name: impl Into<String>,
        code: Code,
    ) -> Result<(), RuntimeError> {
        let name = name.into();
        let code: ModuleCodeString = code.into();

        info!("Adding/updating module: {}", name);

        let mut modules = self.manager.modules();

        modules.insert(
            name.clone(),
            ModuleDefinition {
                code: code.as_str().into(),
            },
        );

        self.manager.update_modules(modules);

        info!("Module {} added/updated successfully", name);
        Ok(())
    }
}

/// Builder for creating a Pool
pub struct PoolBuilder {
    modules: HashMap<String, ModuleDefinition>,
    max_size: usize,
    domain_permission: Option<DomainPermission>,
    evaluation_timeout: Option<std::time::Duration>,
}

impl PoolBuilder {
    /// Create a new PoolBuilder
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            max_size: 10,
            domain_permission: None,
            evaluation_timeout: None,
        }
    }

    /// Set the maximum number of workers in the pool
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Set the evaluation timeout for module loading in all workers
    pub fn with_evaluation_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.evaluation_timeout = Some(timeout);
        self
    }

    /// Set domain permission for all workers and modules
    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = Some(permission);
        self
    }

    /// Add a module to be loaded in all workers
    pub fn add_module<Code: Into<ModuleCodeString>>(
        mut self,
        name: impl Into<String>,
        code: Code,
    ) -> Self {
        let code: ModuleCodeString = code.into();
        self.modules.insert(
            name.into(),
            ModuleDefinition {
                code: code.as_str().into(),
            },
        );
        self
    }

    /// Build the pool
    pub fn build(self) -> Result<Pool, RuntimeError> {
        let cache = SharedCache::new();

        // Construct WorkerOptions from individual fields
        let worker_options = WorkerOptions {
            evaluation_timeout: self.evaluation_timeout.unwrap_or(Duration::from_secs(5)),
            domain_permission: self.domain_permission.unwrap_or(DomainPermission::DenyAll),
        };

        let manager = WorkerManager::new(self.modules, cache, worker_options);

        let pool = deadpool::managed::Pool::builder(manager.clone())
            .max_size(self.max_size)
            .build()
            .map_err(|e| RuntimeError::Unknown(e.to_string()))?;

        Ok(Pool {
            inner: pool,
            manager,
        })
    }
}

impl Default for PoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::OutputChannel;

    use super::*;

    #[tokio::test]
    async fn test_pool_basic() {
        let _ = tracing_subscriber::fmt::try_init();

        let js_code = r#"
            function add(a, b) {
                return a + b;
            }
            export default { add };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("math", js_code.to_string())
            .build()
            .unwrap();

        let result: i32 = pool
            .exec("math", "add", (5, 3), ExecOptions::new())
            .await
            .unwrap_or_else(|e| panic!("Execution failed: {e:?}"));

        assert_eq!(result, 8);
    }

    #[tokio::test]
    async fn test_pool_multiple_modules() {
        let _ = tracing_subscriber::fmt::try_init();

        let add_code = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let multiply_code = r#"
            function multiply(a, b) { return a * b; }
            export default { multiply };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("add", add_code.to_string())
            .add_module("multiply", multiply_code.to_string())
            .build()
            .unwrap();

        let result1: i32 = pool
            .exec(
                "add",
                "add",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let result2: i32 = pool
            .exec(
                "multiply",
                "multiply",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(result1, 8);
        assert_eq!(result2, 15);
    }

    #[tokio::test]
    async fn test_pool_missing_function() {
        let _ = tracing_subscriber::fmt::try_init();

        let add_code = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("add", add_code.to_string())
            .build()
            .unwrap();

        let result_module: Result<i32, RuntimeError> = pool
            .exec(
                "missingModuleTest",
                "add",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await;

        let result_function: Result<i32, RuntimeError> = pool
            .exec(
                "add",
                "missingFunctionTest",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await;

        assert!(result_module.is_err());
        assert!(matches!(
            result_module.unwrap_err(),
            RuntimeError::MissingModule(name) if name == "missingModuleTest"
        ));

        assert!(result_function.is_err());
        // With proper multi-module support, the function name is just the function name
        assert!(matches!(
            result_function.unwrap_err(),
            RuntimeError::MissingExportedFunction(name) if name == "missingFunctionTest"
        ));
    }

    #[tokio::test]
    async fn test_pool_dynamic_module_addition() {
        let _ = tracing_subscriber::fmt::try_init();

        let add_code = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("add", add_code.to_string())
            .build()
            .unwrap();

        // Add a new module dynamically
        let subtract_code = r#"
            function subtract(a, b) { return a - b; }
            export default { subtract };
        "#;

        pool.add_module("subtract", subtract_code.to_string())
            .await
            .unwrap();

        let result: i32 = pool
            .exec(
                "subtract",
                "subtract",
                vec![serde_json::json!(10), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(result, 7);
    }

    #[tokio::test]
    async fn test_pool_multiple_functions_in_one_module() {
        let _ = tracing_subscriber::fmt::try_init();

        // One module with multiple functions!
        let math_utils = r#"
            function add(a, b) { return a + b; }
            function subtract(a, b) { return a - b; }
            function multiply(a, b) { return a * b; }
            function divide(a, b) { return a / b; }
            export default { add, subtract, multiply, divide };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("math_utils", math_utils.to_string())
            .build()
            .unwrap();

        // Call different functions from the same module
        let sum: i32 = pool
            .exec(
                "math_utils",
                "add",
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let difference: i32 = pool
            .exec(
                "math_utils",
                "subtract",
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let product: i32 = pool
            .exec(
                "math_utils",
                "multiply",
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let quotient: i32 = pool
            .exec(
                "math_utils",
                "divide",
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(sum, 15);
        assert_eq!(difference, 5);
        assert_eq!(product, 50);
        assert_eq!(quotient, 2);
    }

    #[tokio::test]
    async fn test_pool_mixed_sync_and_async_functions() {
        let _ = tracing_subscriber::fmt::try_init();

        // Module with both sync and async functions
        let mixed_code = r#"
            function syncAdd(a, b) {
                return a + b;
            }
            
            async function asyncMultiply(a, b) {
                // Simulate async operation
                await new Promise(resolve => setTimeout(resolve, 1));
                return a * b;
            }
            
            export default { syncAdd, asyncMultiply };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("mixed", mixed_code.to_string())
            .build()
            .unwrap();

        let sync_result: i32 = pool
            .exec(
                "mixed",
                "syncAdd",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let async_result: i32 = pool
            .exec(
                "mixed",
                "asyncMultiply",
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(sync_result, 8);
        assert_eq!(async_result, 15);
    }

    #[tokio::test]
    async fn test_pool_shared_cache() {
        let _ = tracing_subscriber::fmt::try_init();

        let js_code = r#"
            function increment() {
                const count = this.context.cache.get("counter") || 0;
                const newCount = count + 1;
                this.context.cache.set("counter", newCount);
                return newCount;
            }
            export default { increment };
        "#;

        let pool = Pool::builder()
            .max_size(3)
            .add_module("counter", js_code.to_string())
            .build()
            .unwrap();

        // Execute multiple times to test the cache across workers
        for i in 1..=10 {
            let result: i32 = pool
                .exec("counter", "increment", (), ExecOptions::new())
                .await
                .unwrap();
            assert_eq!(result, i);
        }
    }

    #[tokio::test]
    async fn test_pool_module_versioning_and_worker_recycling() {
        let _ = tracing_subscriber::fmt::try_init();

        let initial_code = r#"
            function getValue() {
                return 100;
            }
            export default { getValue };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("versioned", initial_code.to_string())
            .build()
            .unwrap();

        assert_eq!(pool.manager.version(), 0);

        let result1: i32 = pool
            .exec("versioned", "getValue", (), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 100);

        // Get a worker to ensure it's in the pool
        let worker = pool.inner.get().await.unwrap();
        let worker_version_before = worker.version();
        assert_eq!(worker_version_before, 0);
        drop(worker);

        // Update the module with new code
        let updated_code = r#"
            function getValue() {
                return 200;
            }
            export default { getValue };
        "#;

        pool.add_module("versioned", updated_code.to_string())
            .await
            .unwrap();

        assert_eq!(pool.manager.version(), 1);

        // Execute with updated code, this should use a new worker or recycled worker
        let result2: i32 = pool
            .exec("versioned", "getValue", (), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result2, 200, "Updated module should return new value");

        // Get a worker and verify it has the new version
        let worker = pool.inner.get().await.unwrap();
        let worker_version_after = worker.version();
        assert_eq!(
            worker_version_after, 1,
            "Worker should have updated version after module update"
        );
        drop(worker);

        let updated_code_v2 = r#"
            function getValue() {
                return 300;
            }
            export default { getValue };
        "#;

        pool.add_module("versioned", updated_code_v2.to_string())
            .await
            .unwrap();

        assert_eq!(pool.manager.version(), 2);

        let result3: i32 = pool
            .exec("versioned", "getValue", (), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result3, 300, "Second update should return newest value");
    }

    #[tokio::test]
    async fn test_domain_permission_deny_all() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { fetchData };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("fetcher", fetch_code.to_string())
            .build()
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "https://fake-domain-for-test.local",
                ExecOptions::new().with_domain_permission(DomainPermission::DenyAll),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::NetworkPermissionDenied(ref msg) if msg.contains("Domain not allowed: https://fake-domain-for-test.local")),
            "Expected NetworkPermissionDenied, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_domain_permission_allow_specific() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { fetchData };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("fetcher", fetch_code.to_string())
            .build()
            .unwrap();

        let allowed_hosts = DomainPermission::Allow(vec!["allowed-domain.test".to_string()]);

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "https://blocked-domain.test",
                ExecOptions::new()
                    .with_domain_permission(allowed_hosts.clone())
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::NetworkPermissionDenied(ref msg) if msg.contains("Allowed domains")),
            "Expected NetworkPermissionDenied with 'Allowed domains' message, got: {err:?}"
        );

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "http://allowed-domain.test",
                ExecOptions::new()
                    .with_domain_permission(allowed_hosts)
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        // it returns error because the domain is invalid (DNS lookup fails), not because it is blacklisted
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::ErrorThrown(ref js_err) if
                js_err.message.as_ref().is_some_and(|msg| msg.contains("failed to lookup address"))
            ),
            "Expected ErrorThrown with DNS/connection error message, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_domain_permission_allow_all() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function testAllowAll(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { testAllowAll };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("fetcher", fetch_code.to_string())
            .build()
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "testAllowAll",
                "https://any-domain.test",
                ExecOptions::new()
                    .with_domain_permission(DomainPermission::AllowAll)
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        // it returns error because the domain is invalid (DNS lookup fails), not because it is blacklisted
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::ErrorThrown(ref js_err) if
                js_err.message.as_ref().is_some_and(|msg| msg.contains("failed to lookup address"))
            ),
            "Expected ErrorThrown with DNS/connection error message, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_domain_permission_deny_specific() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { fetchData };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("fetcher", fetch_code.to_string())
            .build()
            .unwrap();

        let deny_list = DomainPermission::Deny(vec!["blocked-domain.test".to_string()]);

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "https://blocked-domain.test",
                ExecOptions::new()
                    .with_domain_permission(deny_list.clone())
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::NetworkPermissionDenied(ref msg) if msg.contains("deny list")),
            "Expected NetworkPermissionDenied with 'deny list' message, got: {err:?}"
        );

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "http://allowed-domain.test",
                ExecOptions::new()
                    .with_domain_permission(deny_list)
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        // it returns error because the domain is invalid (DNS lookup fails), not because it is blacklisted
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::ErrorThrown(ref js_err) if
                js_err.message.as_ref().is_some_and(|msg| msg.contains("failed to lookup address"))
            ),
            "Expected ErrorThrown with DNS/connection error message, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let _ = tracing_subscriber::fmt::try_init();

        let slow_code = r#"
            async function slowCode() {
                await new Promise(resolve => setTimeout(resolve, 10000));
                return;
            }
            export default { slowCode };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("slow", slow_code.to_string())
            .build()
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "slow",
                "slowCode",
                (),
                ExecOptions::new().with_timeout(Duration::from_millis(10)),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::ExecTimeout),
            "Expected ExecTimeout error, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_exec_options_override_pool_domain_permission() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { fetchData };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .with_domain_permission(DomainPermission::DenyAll)
            .add_module("fetcher", fetch_code.to_string())
            .build()
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "http://foo.bar",
                ExecOptions::new()
                    .with_domain_permission(DomainPermission::AllowAll)
                    .with_timeout(Duration::from_secs(5)),
            )
            .await;

        // it returns error because the domain is invalid (DNS lookup fails), not because it is blacklisted
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::ErrorThrown(ref js_err) if
                js_err.message.as_ref().is_some_and(|msg| msg.contains("failed to lookup address"))
            ),
            "Expected ErrorThrown with DNS/connection error message, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_stdout_streaming() {
        let _ = tracing_subscriber::fmt::try_init();

        let log_code = r#"
            function logAndReturn(a) {
                console.log('test output');
                console.error('test error');
                return a * 2;
            }
            export default { logAndReturn };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("logger", log_code.to_string())
            .build()
            .unwrap();

        let (sender, mut receiver) = tokio::sync::broadcast::channel(16);

        let outputs = Arc::new(tokio::sync::RwLock::new(Vec::new()));
        let fn_outputs = outputs.clone();
        let collect_task = tokio::spawn(async move {
            while let Ok((channel, msg)) = receiver.recv().await {
                let mut v = fn_outputs.write().await;
                match channel {
                    OutputChannel::StdOut => v.push(format!("out {msg}")),
                    OutputChannel::StdErr => v.push(format!("err {msg}")),
                }
                drop(v);
            }
        });

        let result: i32 = pool
            .exec(
                "logger",
                "logAndReturn",
                5,
                ExecOptions::new()
                    .with_timeout(Duration::from_millis(100))
                    .with_stdout_sender(std::sync::Arc::new(sender)),
            )
            .await
            .unwrap();

        assert_eq!(result, 10);

        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(collect_task);

        let outputs = outputs.read().await;
        assert!(!outputs.is_empty(), "Expected some console output");
        assert_eq!(
            outputs.to_vec(),
            vec!["out test output\n", "err test error\n"]
        )
    }
}
