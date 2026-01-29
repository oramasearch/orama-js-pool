use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use std::future::Future;

use crate::orama_extension::SharedCache;

use super::{
    options::WorkerOptions,
    runtime::RuntimeError,
    worker::{Worker, WorkerBuilder},
};

/// Definition of a module to be loaded into workers
#[derive(Clone, Debug)]
pub struct ModuleDefinition {
    pub code: Arc<str>,
}

/// Manager for creating and recycling Workers in the pool
#[derive(Clone)]
pub struct WorkerManager {
    modules: Arc<RwLock<HashMap<String, ModuleDefinition>>>,
    cache: SharedCache,
    pub(crate) worker_options: WorkerOptions,
}

impl WorkerManager {
    pub fn new(
        modules: HashMap<String, ModuleDefinition>,
        cache: SharedCache,
        worker_options: WorkerOptions,
    ) -> Self {
        Self {
            modules: Arc::new(RwLock::new(modules)),
            cache,
            worker_options,
        }
    }

    pub fn update_modules(&self, modules: HashMap<String, ModuleDefinition>) {
        let mut modules_guard = self.modules.write().unwrap();
        *modules_guard = modules;
        drop(modules_guard);
    }

    /// Get a clone of current modules
    pub fn modules(&self) -> HashMap<String, ModuleDefinition> {
        self.modules.read().unwrap().clone()
    }

    /// Get shared cache
    pub fn cache(&self) -> &SharedCache {
        &self.cache
    }
}

impl Manager for WorkerManager {
    type Type = Worker;
    type Error = RuntimeError;

    /// Create a new worker with current module definitions
    fn create(&self) -> impl Future<Output = Result<Self::Type, Self::Error>> + Send {
        let modules = self.modules.read().unwrap().clone();
        let cache = self.cache.clone();
        let worker_options = self.worker_options.clone();

        async move {
            let mut builder = WorkerBuilder::new()
                .with_cache(cache)
                .with_domain_permission(worker_options.domain_permission)
                .with_evaluation_timeout(worker_options.evaluation_timeout);

            for (name, def) in modules {
                builder = builder.add_module(name, def.code);
            }

            let worker = builder.build().await?;

            Ok(worker)
        }
    }

    /// Check if a worker is still healthy
    async fn recycle(
        &self,
        worker: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        if !worker.is_alive() {
            return Err(RecycleError::Message("Worker not alive".into()));
        }

        Ok(())
    }
}
