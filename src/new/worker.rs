use std::collections::HashMap;

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::orama_extension::SharedCache;

use super::{
    options::{ExecOptions, ModuleOptions},
    parameters::TryIntoFunctionParameters,
    runtime::{Runtime, RuntimeError},
};

/// A loaded module with its runtime and metadata
struct LoadedModule {
    runtime: Runtime<serde_json::Value, serde_json::Value>,
    code: ModuleCodeString,
    options: ModuleOptions,
}

/// Worker that can execute multiple modules
pub struct Worker {
    modules: HashMap<String, LoadedModule>,
    cache: SharedCache,
    version: u64,
}

impl Worker {
    /// Create a new worker with the given modules and cache
    pub(crate) fn new(cache: SharedCache, version: u64) -> Self {
        Self {
            modules: HashMap::new(),
            cache,
            version,
        }
    }

    /// Add a module to this worker
    pub async fn add_module<Code>(
        &mut self,
        name: String,
        code: Code,
        options: ModuleOptions,
    ) -> Result<(), RuntimeError>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        let code_string: ModuleCodeString = code.into();
        let (code_for_runtime, code_for_storage) = code_string.into_cheap_copy();

        info!("Loading module: {}", name);

        let runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
            code_for_runtime,
            options.domain_permission.to_allowed_hosts(),
            options.timeout,
            self.cache.clone(),
        )
        .await?;

        let loaded_module = LoadedModule {
            runtime,
            code: code_for_storage,
            options,
        };

        self.modules.insert(name, loaded_module);

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
        let module = self
            .modules
            .get_mut(module_name)
            .ok_or_else(|| RuntimeError::MissingModule(module_name.to_string()))?;

        // Check if runtime is still alive, recreate if needed
        if !module.runtime.is_alive() {
            info!("Runtime not alive, recreating for module: {}", module_name);
            let code = std::mem::replace(&mut module.code, String::new().into());
            let (code_for_runtime, code_for_storage) = code.into_cheap_copy();
            let options = module.options.clone();

            let runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
                code_for_runtime,
                options.domain_permission.to_allowed_hosts(),
                options.timeout,
                self.cache.clone(),
            )
            .await?;

            module.runtime = runtime;
            module.code = code_for_storage;
        }

        module
            .runtime
            .check_function(function_name.to_string(), false)
            .await?;

        let params_tuple = params.try_into_function_parameter()?;
        let params_value = serde_json::to_value(params_tuple.0)?;

        let result: serde_json::Value = module
            .runtime
            .exec(
                function_name.to_string(),
                params_value,
                exec_options.stdout_sender,
                // TODO: fix also the allowd hosts
                None,
                exec_options.timeout,
            )
            .await?;

        // Convert output from serde_json::Value
        let output: Output = serde_json::from_value(result)?;
        Ok(output)
    }

    /// Check if the worker is alive (all runtimes are alive)
    pub fn is_alive(&self) -> bool {
        // Check if at least some modules have alive runtimes
        self.modules.values().any(|m| m.runtime.is_alive())
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
    modules: Vec<(String, ModuleDefinition)>,
    cache: Option<SharedCache>,
    version: Option<u64>,
}

struct ModuleDefinition {
    code: ModuleCodeString,
    options: ModuleOptions,
}

impl WorkerBuilder {
    /// Create a new WorkerBuilder
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            cache: None,
            version: None,
        }
    }

    /// Add a module to the worker
    pub fn add_module<Code: Into<ModuleCodeString>>(
        mut self,
        name: impl Into<String>,
        code: Code,
        options: ModuleOptions,
    ) -> Self {
        let code: ModuleCodeString = code.into();
        self.modules
            .push((name.into(), ModuleDefinition { code, options }));
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

    /// Build the worker
    pub async fn build(self) -> Result<Worker, RuntimeError> {
        let cache = self.cache.unwrap_or_default();
        let version = self.version.unwrap_or(0);

        let mut worker = Worker::new(cache, version);

        for (name, def) in self.modules {
            worker.add_module(name, def.code, def.options).await?;
        }

        Ok(worker)
    }
}

impl Default for WorkerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
