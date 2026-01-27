use std::{collections::HashMap, sync::Arc};

use deno_core::ModuleCodeString;
use serde::de::DeserializeOwned;
use tracing::info;

use crate::{
    orama_extension::{OutputChannel, SharedCache},
    parameters::TryIntoFunctionParameters,
};

use super::{
    options::{ExecOptions, ModuleOptions, ResolvedExecOptions},
    runtime::{Runtime, RuntimeError},
};

/// A loaded module with its runtime and metadata
struct LoadedModule {
    runtime: Runtime<serde_json::Value, serde_json::Value>,
    code: String,
    options: ModuleOptions,
    is_async: bool,
    function_name: String,
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
    pub(crate) async fn add_module<Code>(
        &mut self,
        name: String,
        function_name: String,
        code: Code,
        is_async: bool,
        options: ModuleOptions,
    ) -> Result<(), RuntimeError>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        let code_string: ModuleCodeString = code.into();
        let code_clone = code_string.as_str().to_string();

        info!("Loading module: {}", name);

        let mut runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
            code_string,
            options.domain_permission.to_allowed_hosts(),
            options.timeout,
            self.cache.clone(),
        )
        .await?;

        // Check if function exists
        runtime
            .check_function(function_name.clone(), is_async)
            .await?;

        let loaded_module = LoadedModule {
            runtime,
            code: code_clone,
            options,
            is_async,
            function_name,
        };

        self.modules.insert(name, loaded_module);

        Ok(())
    }

    /// Execute a function in a module
    pub async fn exec<Input, Output>(
        &mut self,
        module_name: &str,
        params: Input,
        exec_options: ExecOptions,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
    ) -> Result<Output, RuntimeError>
    where
        Input: TryIntoFunctionParameters + Send + 'static,
        Output: DeserializeOwned + Send + 'static,
    {
        let module = self
            .modules
            .get_mut(module_name)
            .ok_or_else(|| RuntimeError::NoExportedFunction(module_name.to_string()))?;

        // Merge exec options with module options
        let resolved_options: ResolvedExecOptions =
            exec_options.merge_with_module(&module.options);

        // Check if runtime is still alive, recreate if needed
        if !module.runtime.is_alive() {
            info!("Runtime not alive, recreating for module: {}", module_name);
            let code = module.code.clone();
            let options = module.options.clone();
            let function_name = module.function_name.clone();
            let is_async = module.is_async;

            let mut runtime = Runtime::<serde_json::Value, serde_json::Value>::new(
                code.clone(),
                options.domain_permission.to_allowed_hosts(),
                options.timeout,
                self.cache.clone(),
            )
            .await?;

            runtime.check_function(function_name, is_async).await?;

            module.runtime = runtime;
        }

        // Convert input to serde_json::Value
        let params_tuple = params.try_into_function_parameter()?;
        let params_value = serde_json::to_value(params_tuple.0)?;

        // Execute the function
        let result: serde_json::Value = module
            .runtime
            .exec(
                module.function_name.clone(),
                module.is_async,
                params_value,
                stdout_sender,
                resolved_options.allowed_hosts,
                resolved_options.timeout,
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
}

struct ModuleDefinition {
    code: String,
    function_name: String,
    is_async: bool,
    options: ModuleOptions,
}

impl WorkerBuilder {
    /// Create a new WorkerBuilder
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Add a module to the worker
    pub fn add_module<Code: Into<ModuleCodeString>>(
        mut self,
        name: impl Into<String>,
        function_name: impl Into<String>,
        code: Code,
        is_async: bool,
        options: ModuleOptions,
    ) -> Self {
        let code: ModuleCodeString = code.into();
        self.modules.push((
            name.into(),
            ModuleDefinition {
                code: code.as_str().to_string(),
                function_name: function_name.into(),
                is_async,
                options,
            },
        ));
        self
    }

    /// Build the worker
    pub async fn build(self) -> Result<Worker, RuntimeError> {
        self.build_with_cache(SharedCache::new(), 0).await
    }

    /// Build the worker with a specific cache and version
    pub(crate) async fn build_with_cache(
        self,
        cache: SharedCache,
        version: u64,
    ) -> Result<Worker, RuntimeError> {
        let mut worker = Worker::new(cache, version);

        for (name, def) in self.modules {
            worker
                .add_module(name, def.function_name, def.code, def.is_async, def.options)
                .await?;
        }

        Ok(worker)
    }
}

impl Default for WorkerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
