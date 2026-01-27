use std::collections::HashMap;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::{orama_extension::SharedCache, parameters::TryIntoFunctionParameters};

use super::{
    manager::{ModuleDefinition, WorkerManager},
    options::{ExecOptions, ModuleOptions},
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
        is_async: bool,
        params: Input,
        exec_options: ExecOptions,
    ) -> Result<Output, RuntimeError>
    where
        Input: TryIntoFunctionParameters + Send + 'static,
        Output: DeserializeOwned + Send + 'static,
    {
        let mut worker = self.inner.get().await.map_err(|e| {
            eprintln!("Pool get error: {e:?}");
            RuntimeError::Unknown
        })?;

        // Execute on the worker
        worker.exec(module_name, function_name, is_async, params, exec_options).await
        // Worker is automatically returned to pool when dropped
    }

    /// Add or update a module in the pool
    pub async fn add_module<Code: Into<ModuleCodeString>>(
        &self,
        name: impl Into<String>,
        code: Code,
        options: ModuleOptions,
    ) -> Result<(), RuntimeError> {
        let name = name.into();
        let code: ModuleCodeString = code.into();

        info!("Adding/updating module: {}", name);

        // Get current modules
        let mut modules = self.manager.modules();

        // Add or update the module
        modules.insert(
            name.clone(),
            ModuleDefinition {
                code: code.as_str().to_string(),
                options,
            },
        );

        // Update the manager (this will increment version and invalidate old workers)
        self.manager.update_modules(modules);

        info!("Module {} added/updated successfully", name);
        Ok(())
    }
}

/// Builder for creating a Pool
pub struct PoolBuilder {
    modules: HashMap<String, ModuleDefinition>,
    max_size: usize,
}

impl PoolBuilder {
    /// Create a new PoolBuilder
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            max_size: 10,
        }
    }

    /// Set the maximum number of workers in the pool
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Add a module to be loaded in all workers
    pub fn add_module<Code: Into<ModuleCodeString>>(
        mut self,
        name: impl Into<String>,
        code: Code,
        options: ModuleOptions,
    ) -> Self {
        let code: ModuleCodeString = code.into();
        self.modules.insert(
            name.into(),
            ModuleDefinition {
                code: code.as_str().to_string(),
                options,
            },
        );
        self
    }

    /// Build the pool
    pub fn build(self) -> Result<Pool, RuntimeError> {
        let cache = SharedCache::new();
        let manager = WorkerManager::new(self.modules.clone(), cache);

        let pool = deadpool::managed::Pool::builder(manager.clone())
            .max_size(self.max_size)
            .build()
            .map_err(|e| {
                eprintln!("Failed to build pool: {e:?}");
                RuntimeError::Unknown
            })?;

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
    use super::*;
    use std::time::Duration;

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
            .add_module(
                "math",
                js_code.to_string(),
                ModuleOptions {
                    timeout: Duration::from_secs(5),
                    domain_permission: super::super::options::DomainPermission::Deny,
                },
            )
            .build()
            .unwrap();

        let result: i32 = pool
            .exec(
                "math",
                "add",
                false,
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
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
            .add_module(
                "add",
                add_code.to_string(),
                ModuleOptions::default(),
            )
            .add_module(
                "multiply",
                multiply_code.to_string(),
                ModuleOptions::default(),
            )
            .build()
            .unwrap();

        let result1: i32 = pool
            .exec(
                "add",
                "add",
                false,
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let result2: i32 = pool
            .exec(
                "multiply",
                "multiply",
                false,
                vec![serde_json::json!(5), serde_json::json!(3)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(result1, 8);
        assert_eq!(result2, 15);
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
            .add_module(
                "add",
                add_code.to_string(),
                ModuleOptions::default(),
            )
            .build()
            .unwrap();

        // Add a new module dynamically
        let subtract_code = r#"
            function subtract(a, b) { return a - b; }
            export default { subtract };
        "#;

        pool.add_module(
            "subtract",
            subtract_code.to_string(),
            ModuleOptions::default(),
        )
        .await
        .unwrap();

        let result: i32 = pool
            .exec(
                "subtract",
                "subtract",
                false,
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
            .add_module(
                "math_utils",
                math_utils.to_string(),
                ModuleOptions::default(),
            )
            .build()
            .unwrap();

        // Call different functions from the same module
        let sum: i32 = pool
            .exec(
                "math_utils",
                "add",
                false,
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let difference: i32 = pool
            .exec(
                "math_utils",
                "subtract",
                false,
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let product: i32 = pool
            .exec(
                "math_utils",
                "multiply",
                false,
                vec![serde_json::json!(10), serde_json::json!(5)],
                ExecOptions::new(),
            )
            .await
            .unwrap();

        let quotient: i32 = pool
            .exec(
                "math_utils",
                "divide",
                false,
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
            .add_module(
                "counter",
                js_code.to_string(),
                ModuleOptions::default(),
            )
            .build()
            .unwrap();

        // Execute multiple times - cache should be shared across workers
        for i in 1..=10 {
            let result: i32 = pool.exec("counter", "increment", false, (), ExecOptions::new()).await.unwrap();
            assert_eq!(result, i);
        }
    }
}
