use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use std::future::Future;

use tracing::info;

use crate::orama_extension::SharedCache;

use super::{
    options::ModuleOptions,
    runtime::RuntimeError,
    worker::{Worker, WorkerBuilder},
};

/// Definition of a module to be loaded into workers
#[derive(Clone)]
pub struct ModuleDefinition {
    pub code: Arc<str>,
    pub options: ModuleOptions,
}

/// Manager for creating and recycling Workers in the pool
#[derive(Clone)]
pub struct WorkerManager {
    modules: Arc<RwLock<HashMap<String, ModuleDefinition>>>,
    cache: SharedCache,
    version: Arc<AtomicU64>,
}

impl WorkerManager {
    /// Create a new WorkerManager
    pub fn new(modules: HashMap<String, ModuleDefinition>, cache: SharedCache) -> Self {
        Self {
            modules: Arc::new(RwLock::new(modules)),
            cache,
            version: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Update modules and increment version
    pub fn update_modules(&self, modules: HashMap<String, ModuleDefinition>) {
        let mut modules_guard = self.modules.write().unwrap();
        *modules_guard = modules;
        drop(modules_guard);

        // Increment version to invalidate existing workers
        self.version.fetch_add(1, Ordering::Release);
        info!(
            "Modules updated, version incremented to {}",
            self.version.load(Ordering::Acquire)
        );
    }

    /// Get current module version
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
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
        let version = self.version.load(Ordering::Acquire);
        let cache = self.cache.clone();

        async move {
            info!("Creating new worker");

            let mut builder = WorkerBuilder::new();

            for (name, def) in modules {
                builder = builder.add_module(name, def.code, def.options);
            }

            let worker = builder
                .with_cache(cache)
                .with_version(version)
                .build()
                .await?;

            info!("Worker created successfully with version {}", version);
            Ok(worker)
        }
    }

    /// Check if a worker is still healthy and has the correct version
    fn recycle(
        &self,
        worker: &mut Self::Type,
        _metrics: &Metrics,
    ) -> impl Future<Output = RecycleResult<Self::Error>> + Send {
        let current_version = self.version.load(Ordering::Acquire);
        let worker_version = worker.version();

        async move {
            // Check version mismatch
            if worker_version != current_version {
                info!(
                    "Worker version mismatch: worker={}, current={}. Will recreate.",
                    worker_version, current_version
                );
                return Err(RecycleError::Message("Module version mismatch".into()));
            }

            // Check if worker is alive
            if !worker.is_alive() {
                info!("Worker not alive. Will recreate.");
                return Err(RecycleError::Message("Worker not alive".into()));
            }

            // Worker is healthy and up-to-date
            Ok(())
        }
    }
}
