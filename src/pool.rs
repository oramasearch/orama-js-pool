use std::collections::HashMap;
use std::time::Duration;

use deadpool::managed::Object;
use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;

use crate::{orama_extension::SharedCache, RecyclePolicy};

use super::{
    manager::{ModuleDefinition, WorkerManager},
    options::{DomainPermission, ExecOptions, MaxExecutions, WorkerOptions},
    parameters::TryIntoFunctionParameters,
    runtime::RuntimeError,
    worker::WorkerBuilder,
};

/// A pool of JavaScript workers that can execute functions from loaded modules.
///
/// Pool is `Clone`, `Send`, and `Sync`, making it safe to share across threads
/// and async tasks. Cloning is cheap as it only clones Arc pointers internally.
#[derive(Clone)]
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
        params: &Input,
        exec_options: ExecOptions,
    ) -> Result<Output, RuntimeError>
    where
        Input: TryIntoFunctionParameters + Send + Sync + 'static + ?Sized,
        Output: DeserializeOwned + Send + 'static,
    {
        let mut worker = self.get_worker().await?;
        let result = worker
            .exec(module_name, function_name, params, exec_options)
            .await;

        if let Err(ref err) = result {
            let recycle_policy = self.manager.worker_options.recycle_policy;

            let should_discard = match recycle_policy {
                RecyclePolicy::Never => false,
                RecyclePolicy::OnTimeout => {
                    matches!(err, RuntimeError::ExecTimeout)
                }
                RecyclePolicy::OnError => !matches!(err, RuntimeError::NetworkPermissionDenied(_)),
                RecyclePolicy::OnTimeoutOrError => {
                    matches!(err, RuntimeError::ExecTimeout)
                        || !matches!(err, RuntimeError::NetworkPermissionDenied(_))
                }
            };

            if should_discard {
                // Take the worker out so it won't be returned to the pool
                // This will cause the worker to be dropped and a new one created on next use
                let _ = deadpool::managed::Object::take(worker);
            }
        }

        result
    }

    /// Add or update a module in the pool
    pub async fn add_module<Code: Into<ModuleCodeString>>(
        &self,
        name: impl Into<String>,
        code: Code,
    ) -> Result<(), RuntimeError> {
        let name = name.into();
        let code: ModuleCodeString = code.into();
        let (runtime_code, module_code) = code.into_cheap_copy();

        let worker = self.get_worker().await?;
        // Take the worker out so it won't be returned to the pool
        // This will cause the worker to be dropped and a new one created on next use
        // So we do not create inconsistent workers.
        let mut worker = deadpool::managed::Object::take(worker);
        worker.add_module(name.clone(), runtime_code).await?;

        let mut modules = self.manager.modules();

        modules.insert(
            name,
            ModuleDefinition {
                code: module_code.as_str().into(),
            },
        );

        self.manager.update_modules(modules);

        Ok(())
    }

    /// Remove a module from the pool
    pub async fn remove_module(&self, name: impl Into<String>) -> Result<(), RuntimeError> {
        let name = name.into();

        let mut modules = self.manager.modules();

        if !modules.contains_key(&name) {
            return Err(RuntimeError::MissingModule(name));
        }

        modules.remove(&name);
        self.manager.update_modules(modules);

        Ok(())
    }

    async fn get_worker(&self) -> Result<Object<WorkerManager>, RuntimeError> {
        self.inner.get().await.map_err(|e| match e {
            deadpool::managed::PoolError::Backend(err) => match err {
                // The recycle policy of the poll will prevent this
                RuntimeError::Terminated => unreachable!(),
                _ => err,
            },
            _ => RuntimeError::Unknown(e.to_string()),
        })
    }
}

/// Builder for creating a Pool.
pub struct PoolBuilder {
    modules: HashMap<String, ModuleDefinition>,
    max_size: usize,
    domain_permission: Option<DomainPermission>,
    evaluation_timeout: Option<std::time::Duration>,
    execution_timeout: Option<std::time::Duration>,
    recycle_policy: Option<RecyclePolicy>,
    max_executions: Option<MaxExecutions>,
}

impl PoolBuilder {
    /// Create a new PoolBuilder
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            max_size: 10,
            domain_permission: None,
            evaluation_timeout: None,
            execution_timeout: None,
            recycle_policy: None,
            max_executions: None,
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

    /// Set the execution timeout for function execution in all workers
    pub fn with_execution_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.execution_timeout = Some(timeout);
        self
    }

    /// Set domain permission for all workers and modules
    pub fn with_domain_permission(mut self, permission: DomainPermission) -> Self {
        self.domain_permission = Some(permission);
        self
    }

    /// Set the recycle policy for all workers
    pub fn with_recycle_policy(mut self, policy: RecyclePolicy) -> Self {
        self.recycle_policy = Some(policy);
        self
    }

    /// Set the maximum number of executions before recycling workers in the pool.
    pub fn with_max_executions(mut self, max: MaxExecutions) -> Self {
        self.max_executions = Some(max);
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
    pub async fn build(self) -> Result<Pool, RuntimeError> {
        let cache = SharedCache::new();

        let worker_options = WorkerOptions {
            evaluation_timeout: self.evaluation_timeout.unwrap_or(Duration::from_secs(5)),
            execution_timeout: self.execution_timeout.unwrap_or(Duration::from_secs(30)),
            domain_permission: self.domain_permission.unwrap_or_default(),
            recycle_policy: self.recycle_policy.unwrap_or_default(),
            max_executions: self.max_executions.unwrap_or_default(),
        };

        // Validate that all modules can be loaded within the evaluation timeout
        // by creating a test worker
        if !self.modules.is_empty() {
            let mut builder = WorkerBuilder::new()
                .with_cache(cache.clone())
                .with_domain_permission(worker_options.domain_permission.clone())
                .with_evaluation_timeout(worker_options.evaluation_timeout);

            for (name, def) in &self.modules {
                builder = builder.add_module(name, def.code.clone());
            }

            // This will fail if any module takes longer than evaluation_timeout to load
            builder.build().await?;
        }

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

    use crate::{OutputChannel, RecyclePolicy};

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
            .await
            .unwrap();

        let result: i32 = pool
            .exec("math", "add", &(5, 3), ExecOptions::new())
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
            .await
            .unwrap();

        let result1: i32 = pool
            .exec(
                "add",
                "add",
                &vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let result2: i32 = pool
            .exec(
                "multiply",
                "multiply",
                &vec![serde_json::json!(5), serde_json::json!(3)],
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
            .await
            .unwrap();

        let result_module: Result<i32, RuntimeError> = pool
            .exec(
                "missingModuleTest",
                "add",
                &vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await;

        let result_function: Result<i32, RuntimeError> = pool
            .exec(
                "add",
                "missingFunctionTest",
                &vec![serde_json::json!(5), serde_json::json!(3)],
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
        let err = result_function.unwrap_err();
        assert!(
            matches!(
            &err,
                RuntimeError::MissingExportedFunction(name) if name == "missingFunctionTest"
            ),
            "Instead got error: {err}"
        );
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
            .await
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
                &vec![serde_json::json!(10), serde_json::json!(3)],
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
            .await
            .unwrap();

        // Call different functions from the same module
        let sum: i32 = pool
            .exec(
                "math_utils",
                "add",
                &vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let difference: i32 = pool
            .exec(
                "math_utils",
                "subtract",
                &vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let product: i32 = pool
            .exec(
                "math_utils",
                "multiply",
                &vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let quotient: i32 = pool
            .exec(
                "math_utils",
                "divide",
                &vec![serde_json::json!(10), serde_json::json!(5)],
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
            .await
            .unwrap();

        let sync_result: i32 = pool
            .exec(
                "mixed",
                "syncAdd",
                &vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let async_result: i32 = pool
            .exec(
                "mixed",
                "asyncMultiply",
                &vec![serde_json::json!(5), serde_json::json!(3)],
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
            .await
            .unwrap();

        // Execute multiple times to test the cache across workers
        for i in 1..=10 {
            let result: i32 = pool
                .exec("counter", "increment", &(), ExecOptions::new())
                .await
                .unwrap();
            assert_eq!(result, i);
        }
    }

    #[tokio::test]
    async fn test_pool_add_module() {
        let _ = tracing_subscriber::fmt::try_init();

        let initial_code = r#"
            function getValue() {
                return 100;
            }
            export default { getValue };
        "#;

        let pool = Pool::builder()
            .max_size(3)
            .add_module("versioned", initial_code.to_string())
            .build()
            .await
            .unwrap();

        let result1: i32 = pool
            .exec("versioned", "getValue", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 100);

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

        // Execute with updated code, this should use a new worker or recycled worker
        let result2: i32 = pool
            .exec("versioned", "getValue", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result2, 200, "Updated module should return new value");

        let updated_code_v2 = r#"
            function getValue() {
                return 300;
            }
            export default { getValue };
        "#;

        pool.add_module("versioned", updated_code_v2.to_string())
            .await
            .unwrap();

        // Test all the instances concurrently
        let mut handles = vec![];
        for _ in 0..10 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let result: i32 = pool_clone
                    .exec("versioned", "getValue", &(), ExecOptions::new())
                    .await
                    .unwrap();
                result
            });
            handles.push(handle);
        }

        // Wait for all tasks and verify results
        for handle in handles {
            let result = handle.await.unwrap();
            assert_eq!(result, 300, "Second update should return newest value");
        }
    }

    #[tokio::test]
    async fn test_pool_add_module_old_workers_stay_in_pool() {
        let _ = tracing_subscriber::fmt::try_init();

        let initial_code = r#"
            async function getValue() {
                await new Promise(resolve => setTimeout(resolve, 10));
                return 100;
            }
            export default { getValue };
        "#;

        let pool = Pool::builder()
            .max_size(5)
            .add_module("versioned", initial_code.to_string())
            .build()
            .await
            .unwrap();

        // Fill the pool with 5 workers that have the old code
        let mut handles = vec![];
        for _ in 0..100 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let result: i32 = pool_clone
                    .exec("versioned", "getValue", &(), ExecOptions::new())
                    .await
                    .unwrap();
                result
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert_eq!(result, 100, "Should return initial value");
        }

        // Now update the module
        let updated_code = r#"
            async function getValue() {
                await new Promise(resolve => setTimeout(resolve, 10));
                return 200;
            }
            export default { getValue };
        "#;

        pool.add_module("versioned", updated_code.to_string())
            .await
            .unwrap();

        // Immediately run 10 concurrent executions
        // If old workers are still in the pool, we should see some return 100
        let mut handles = vec![];
        for _ in 0..100 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let result: i32 = pool_clone
                    .exec("versioned", "getValue", &(), ExecOptions::new())
                    .await
                    .unwrap();
                result
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            let result = handle.await.unwrap();
            results.push(result);
        }

        // Check if any old workers (returning 100) are still in the pool
        let old_values = results.iter().filter(|&&v| v == 100).count();
        let new_values = results.iter().filter(|&&v| v == 200).count();

        assert_eq!(
            old_values, 0,
            "Found {old_values} old workers still in pool! Old workers should be removed.",
        );
        assert_eq!(new_values, 100, "All workers should have the new code");
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
            .await
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
            .await
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
            .await
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
            .await
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
    async fn test_domain_permission_worker_default() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return await response.text();
            }
            export default { fetchData };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("fetcher", fetch_code.to_string())
            .with_domain_permission(DomainPermission::AllowAll)
            .build()
            .await
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "https://blocked-domain.test",
                ExecOptions::new()
                    .with_domain_permission(DomainPermission::DenyAll)
                    .with_timeout(Duration::from_secs(2)),
            )
            .await;

        // use exec deny list
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, RuntimeError::NetworkPermissionDenied(_)));

        let result: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchData",
                "http://allowed-domain.test",
                // No specification of the domain permission, so it should use the worker one.
                ExecOptions::new().with_timeout(Duration::from_secs(2)),
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
            .await
            .unwrap();

        let result: Result<String, RuntimeError> = pool
            .exec(
                "slow",
                "slowCode",
                &(),
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
            .await
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
            .await
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
                &5,
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

    #[tokio::test]
    async fn test_not_recreate_instance_on_error() {
        let _ = tracing_subscriber::fmt::try_init();

        let error_code = r#"
            let callCount = 0;
            function maybeError(shouldError) {
                if (shouldError) {
                    throw new Error("Test error");
                }
                callCount++;
                return callCount;
            }
            export default { maybeError };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("error_test", error_code.to_string())
            .with_recycle_policy(RecyclePolicy::Never)
            .build()
            .await
            .unwrap();

        let result1: i32 = pool
            .exec("error_test", "maybeError", &false, ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 1);

        let result2: Result<i32, RuntimeError> = pool
            .exec("error_test", "maybeError", &true, ExecOptions::new())
            .await;
        assert!(result2.is_err());

        let result3: i32 = pool
            .exec("error_test", "maybeError", &false, ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result3, 2, "Runtime should have been recycled after error");
    }

    #[tokio::test]
    async fn test_recycle_policy_on_error() {
        let _ = tracing_subscriber::fmt::try_init();

        let error_code = r#"
            let callCount = 0;
            function maybeError(shouldError) {
                callCount++;
                if (shouldError) {
                    throw new Error("Test error");
                }
                return callCount;
            }
            export default { maybeError };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("error_test", error_code.to_string())
            .with_recycle_policy(RecyclePolicy::OnError)
            .build()
            .await
            .unwrap();

        // First call - should succeed
        let result1: i32 = pool
            .exec("error_test", "maybeError", &false, ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 1);

        // Second call with error - should fail
        let result2: Result<i32, RuntimeError> = pool
            .exec("error_test", "maybeError", &true, ExecOptions::new())
            .await;
        assert!(result2.is_err());

        // Third call - should succeed with fresh runtime (callCount reset to 1)
        let result3: i32 = pool
            .exec("error_test", "maybeError", &false, ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result3, 1, "Runtime should have been recycled after error");
    }

    #[tokio::test]
    async fn test_recycle_policy_timeout_or_error() {
        let _ = tracing_subscriber::fmt::try_init();

        let counter_code = r#"
            let callCount = 0;
            function incrementCounter() {
                callCount++;
                if (callCount == 2){
                    throw new Error("error");
                }
                return callCount;
            }
            export default { incrementCounter };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("counter", counter_code.to_string())
            .with_recycle_policy(RecyclePolicy::OnTimeoutOrError)
            .build()
            .await
            .unwrap();

        let result: i32 = pool
            .exec("counter", "incrementCounter", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result, 1);

        // Second call should error (callCount == 2)
        let result: Result<i32, RuntimeError> = pool
            .exec("counter", "incrementCounter", &(), ExecOptions::new())
            .await;
        assert!(result.is_err(), "Should error on second call");

        // Third call should succeed with fresh runtime (callCount reset to 1)
        let result: i32 = pool
            .exec("counter", "incrementCounter", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result, 1, "Runtime should have been recycled after error");
    }

    #[tokio::test]
    async fn test_recycle_policy_on_error_with_network_denial() {
        let _ = tracing_subscriber::fmt::try_init();

        let fetch_code = r#"
            let callCount = 0;
            async function fetchOrCount(url) {
                callCount++;
                if (url) {
                    const response = await fetch(url);
                    return await response.text();
                }
                return callCount;
            }
            export default { fetchOrCount };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("fetcher", fetch_code.to_string())
            .with_domain_permission(DomainPermission::Deny(vec!["blocked.test".to_string()]))
            .with_recycle_policy(RecyclePolicy::OnError)
            .build()
            .await
            .unwrap();

        let result1: i32 = pool
            .exec(
                "fetcher",
                "fetchOrCount",
                &(None::<String>,),
                ExecOptions::new(),
            )
            .await
            .unwrap();
        assert_eq!(result1, 1);

        // Second call - with blocked URL, should fail with network permission error
        let result2: Result<String, RuntimeError> = pool
            .exec(
                "fetcher",
                "fetchOrCount",
                &("https://blocked.test",),
                ExecOptions::new().with_timeout(Duration::from_secs(2)),
            )
            .await;

        assert!(result2.is_err(), "Should error on blocked domain");
        assert!(
            matches!(
                result2.unwrap_err(),
                RuntimeError::NetworkPermissionDenied(_)
            ),
            "Should be NetworkPermissionDenied error"
        );

        let result3: i32 = pool
            .exec(
                "fetcher",
                "fetchOrCount",
                &(None::<String>,),
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(
            result3, 3,
            "Runtime should NOT have been recycled after network permission denial (permission errors are not runtime errors)"
        );
    }

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

        let pool = Pool::builder()
            .max_size(1)
            .with_evaluation_timeout(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        let result = pool
            .add_module("expensive", expensive_code.to_string())
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        assert!(matches!(result.unwrap_err(), RuntimeError::InitTimeout));

        // also on adding a module
        let result = pool
            .add_module("expensive_2", expensive_code.to_string())
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        assert!(matches!(result.unwrap_err(), RuntimeError::InitTimeout));

        // not expensive
        let add_code = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;
        let result = pool.add_module("not_expensive", add_code.to_string()).await;
        assert!(
            result.is_ok(),
            "Should not error: {:?}",
            result.unwrap_err()
        );
    }

    #[tokio::test]
    async fn test_module_evaluation_timeout_builder() {
        let _ = tracing_subscriber::fmt::try_init();

        let expensive_code = r#"
            await new Promise(resolve => setTimeout(resolve, 10000));
            function getValue() {
                return 42;
            }
            export default { getValue };
        "#;

        let result = Pool::builder()
            .max_size(1)
            .with_evaluation_timeout(Duration::from_millis(100))
            .add_module("expensive", expensive_code.to_string())
            .build()
            .await;

        assert!(result.is_err(), "Module evaluation should timeout");
        match result {
            Ok(_) => todo!(),
            Err(e) => assert!(matches!(e, RuntimeError::InitTimeout)),
        }
    }

    #[tokio::test]
    async fn test_pool_remove_module() {
        let _ = tracing_subscriber::fmt::try_init();

        let code2 = r#"
            function add(a, b) { return a + b; }
            export default { add };
        "#;

        let code1 = r#"
            function getValue() { return 100; }
            export default { getValue };
        "#;

        let pool = Pool::builder()
            .max_size(2)
            .add_module("module2", code2.to_string())
            .add_module("module1", code1.to_string())
            .build()
            .await
            .unwrap();

        let result1: i32 = pool
            .exec("module1", "getValue", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 100);

        let result2: i32 = pool
            .exec("module2", "add", &(5, 3), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result2, 8);

        pool.remove_module("module1").await.unwrap();

        let modules = pool.manager.modules();
        assert!(
            !modules.contains_key("module1"),
            "Module should be removed from manager"
        );
        assert!(
            modules.contains_key("module2"),
            "Module2 should still be in manager"
        );

        let result2: i32 = pool
            .exec("module2", "add", &(10, 5), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result2, 15);

        let result2: Result<i32, RuntimeError> = pool
            .exec("module1", "getValue", &(10, 5), ExecOptions::new())
            .await;

        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_pool_remove_nonexistent_module() {
        let _ = tracing_subscriber::fmt::try_init();

        let pool = Pool::builder().max_size(2).build().await.unwrap();

        // Removing a module that doesn't exist should fail
        let result = pool.remove_module("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeError::MissingModule(name) if name == "nonexistent"
        ));
    }

    #[tokio::test]
    async fn test_pool_max_executions() {
        let _ = tracing_subscriber::fmt::try_init();

        let counter_code = r#"
            let callCount = 0;
            function increment() {
                callCount++;
                return callCount;
            }
            export default { increment };
        "#;

        let pool = Pool::builder()
            .max_size(1)
            .add_module("counter", counter_code.to_string())
            .with_max_executions(MaxExecutions::Limited(2))
            .build()
            .await
            .unwrap();

        // First 3 executions should increment the counter
        let result1: i32 = pool
            .exec("counter", "increment", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result1, 1);

        let result2: i32 = pool
            .exec("counter", "increment", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result2, 2);

        // After 2 executions, the worker should be recycled
        // and the counter should reset to 1
        let result4: i32 = pool
            .exec("counter", "increment", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(
            result4, 1,
            "Counter should reset after max_executions is reached"
        );

        // Verify it continues to work correctly after recycling
        let result5: i32 = pool
            .exec("counter", "increment", &(), ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result5, 2);
    }

    #[tokio::test]
    async fn test_pool_execution_timeout_priority() {
        let _ = tracing_subscriber::fmt::try_init();

        let slow_code = r#"
            async function slowCode(delay) {
                await new Promise(resolve => setTimeout(resolve, delay));
                return "completed";
            }
            export default { slowCode };
        "#;

        // Pool has 5 second timeout
        let pool = Pool::builder()
            .max_size(2)
            .with_execution_timeout(Duration::from_secs(2))
            .add_module("slow", slow_code.to_string())
            .build()
            .await
            .unwrap();

        // Case 1: No ExecOptions timeout - uses pool timeout (5 seconds)
        let result: String = pool
            .exec("slow", "slowCode", &100, ExecOptions::new())
            .await
            .unwrap();
        assert_eq!(result, "completed");

        // Case 2: ExecOptions timeout set - overrides pool timeout
        let result: Result<String, RuntimeError> = pool
            .exec(
                "slow",
                "slowCode",
                &200,
                ExecOptions::new().with_timeout(Duration::from_millis(50)),
            )
            .await;
        assert!(matches!(result.unwrap_err(), RuntimeError::ExecTimeout));

        // Case 3: ExecOptions with longer timeout - overrides pool timeout
        let result: String = pool
            .exec(
                "slow",
                "slowCode",
                &2500,
                ExecOptions::new().with_timeout(Duration::from_secs(3)),
            )
            .await
            .unwrap();
        assert_eq!(result, "completed");
    }
}
