use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use deno_core::ModuleCodeString;
use tokio::sync::RwLock;

use crate::orama_extension::{OutputChannel, SharedCache, SharedKV, SharedSecrets};
use crate::runner::ExecOption;
use crate::{executor::JSExecutor, JSRunnerError, TryIntoFunctionParameters};
use std::collections::HashMap;

pub struct JSPoolExecutor<Input, Output> {
    executors: Arc<Vec<RwLock<JSExecutor<Input, Output>>>>,
    index: AtomicUsize,
    shared_kv: SharedKV,
    shared_secrets: SharedSecrets,
}

pub struct JSPoolExecutorConfig {
    pub instances: usize,
    pub queue_capacity: usize,
    pub allowed_hosts_on_init: Option<Vec<String>>,
    pub timeout_on_init: std::time::Duration,
    pub is_async: bool,
    pub function_name: String,
}

impl<Input: TryIntoFunctionParameters, Output: serde::de::DeserializeOwned + 'static>
    JSPoolExecutor<Input, Output>
{
    #[allow(clippy::too_many_arguments)]
    pub async fn new<Code: Into<ModuleCodeString> + Send + Clone + 'static>(
        code: Code,
        instances: usize,
        kv: Option<HashMap<String, String>>,
        secrets: Option<HashMap<String, String>>,
        allowed_hosts_on_init: Option<Vec<String>>,
        timeout_on_init: std::time::Duration,
        is_async: bool,
        function_name: String,
    ) -> Result<Self, JSRunnerError> {
        // Create shared instances
        let shared_cache = SharedCache::new();
        let shared_kv = match kv {
            Some(map) => SharedKV::from_map(map),
            None => SharedKV::new(),
        };
        let shared_secrets = match secrets {
            Some(map) => SharedSecrets::from_map(map),
            None => SharedSecrets::new(),
        };

        let mut executors = Vec::with_capacity(instances);
        for _ in 0..(instances) {
            let executor = JSExecutor::try_new(
                code.clone(),
                allowed_hosts_on_init.clone(),
                timeout_on_init,
                is_async,
                function_name.clone(),
                shared_cache.clone(),
                shared_kv.clone(),
                shared_secrets.clone(),
            )
            .await?;
            let executor = RwLock::new(executor);
            executors.push(executor);
        }

        let executors = Arc::new(executors);

        Ok(Self {
            executors,
            index: AtomicUsize::new(0),
            shared_kv,
            shared_secrets,
        })
    }

    /// Create a builder for JSPoolExecutor
    pub fn builder<Code>(
        code: Code,
        function_name: impl Into<String>,
    ) -> JSPoolExecutorBuilder<Input, Output, Code>
    where
        Code: Into<ModuleCodeString> + Send + Clone + 'static,
    {
        JSPoolExecutorBuilder::new(code, function_name)
    }

    /// Update a KV value (thread-safe, available to all executors)
    pub fn update_kv(&self, key: String, value: String) {
        self.shared_kv.set(key, value);
    }

    /// Delete a KV entry (thread-safe, affects all executors)
    pub fn delete_kv(&self, key: &str) {
        self.shared_kv.delete(key);
    }

    /// Update a Secret value (thread-safe, available to all executors)
    pub fn update_secret(&self, key: String, value: String) {
        self.shared_secrets.set(key, value);
    }

    /// Delete a Secret entry (thread-safe, affects all executors)
    pub fn delete_secret(&self, key: &str) {
        self.shared_secrets.delete(key);
    }

    pub async fn exec(
        &self,
        params: Input,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        option: ExecOption,
    ) -> Result<Output, JSRunnerError> {
        let index = self.index.fetch_add(1, Ordering::AcqRel);
        let executor = &self.executors[index % self.executors.len()];
        let mut executor_lock = executor.write().await;
        executor_lock.exec(params, stdout_sender, option).await
    }
}

/// Builder for JSPoolExecutor
pub struct JSPoolExecutorBuilder<Input, Output, Code> {
    code: Code,
    function_name: String,
    instances: usize,
    kv: Option<HashMap<String, String>>,
    secrets: Option<HashMap<String, String>>,
    allowed_hosts_on_init: Option<Vec<String>>,
    timeout_on_init: std::time::Duration,
    is_async: bool,
    _phantom: std::marker::PhantomData<(Input, Output)>,
}

impl<Input: TryIntoFunctionParameters, Output: serde::de::DeserializeOwned + 'static, Code>
    JSPoolExecutorBuilder<Input, Output, Code>
where
    Code: Into<ModuleCodeString> + Send + Clone + 'static,
{
    pub fn new(code: Code, function_name: impl Into<String>) -> Self {
        Self {
            code,
            function_name: function_name.into(),
            instances: 1,
            kv: None,
            secrets: None,
            allowed_hosts_on_init: Some(vec![]), // Default: no network access
            timeout_on_init: std::time::Duration::from_secs(30),
            is_async: false,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn instances(mut self, instances: usize) -> Self {
        self.instances = instances;
        self
    }

    pub fn kv(mut self, kv: HashMap<String, String>) -> Self {
        self.kv = Some(kv);
        self
    }

    pub fn secrets(mut self, secrets: HashMap<String, String>) -> Self {
        self.secrets = Some(secrets);
        self
    }

    pub fn allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts_on_init = Some(hosts);
        self
    }

    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout_on_init = timeout;
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    pub async fn build(self) -> Result<JSPoolExecutor<Input, Output>, JSRunnerError> {
        JSPoolExecutor::new(
            self.code,
            self.instances,
            self.kv,
            self.secrets,
            self.allowed_hosts_on_init,
            self.timeout_on_init,
            self.is_async,
            self.function_name,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;
    use std::time::Duration;

    #[derive(Clone, Serialize, Deserialize)]
    struct TestInput(i32);

    #[tokio::test]
    async fn test_jspool_executor_basic() {
        // JS code: function addOne(x) { return x + 1; }
        let js_code = r#"
            function addOne(x) { return x + 1; }
            export default { addOne };
        "#
        .to_string();

        let pool = JSPoolExecutor::<TestInput, i32>::new(
            js_code,
            2,    // two executors
            None, // No KV
            None, // No Secrets
            None,
            Duration::from_secs(2),
            false,
            "addOne".to_string(),
        )
        .await
        .expect("Failed to create JSPoolExecutor");

        let result = pool
            .exec(
                TestInput(41),
                None,
                ExecOption {
                    timeout: Duration::from_secs(2),
                    allowed_hosts: None,
                },
            )
            .await
            .expect("Execution failed");

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_kv_and_secrets() {
        let js_code = r#"
            function get_config() {
                const endpoint = this.orama.kv.get("api_endpoint");
                const key = this.orama.secret.get("api_key");
                return `${endpoint}:${key}`;
            }
            export default { get_config };
        "#
        .to_string();

        let mut kv_map = HashMap::new();
        kv_map.insert(
            "api_endpoint".to_string(),
            "https://old-api.example.com".to_string(),
        );

        let mut secrets_map = HashMap::new();
        secrets_map.insert("api_key".to_string(), "old_secret".to_string());

        let pool = JSPoolExecutor::<(), String>::new(
            js_code,
            2, // Multiple executor to test kv/secret across the pool
            Some(kv_map),
            Some(secrets_map),
            None,
            Duration::from_secs(2),
            false,
            "get_config".to_string(),
        )
        .await
        .expect("Failed to create JSPoolExecutor");

        // First execution - old values
        let result = pool
            .exec(
                (),
                None,
                ExecOption {
                    timeout: Duration::from_secs(2),
                    allowed_hosts: None,
                },
            )
            .await
            .expect("Execution failed");

        assert_eq!(result, "https://old-api.example.com:old_secret");

        // Update
        pool.update_kv(
            "api_endpoint".to_string(),
            "https://new-api.example.com".to_string(),
        );
        pool.update_secret("api_key".to_string(), "new_secret_456".to_string());

        // Second execution - new values
        let result = pool
            .exec(
                (),
                None,
                ExecOption {
                    timeout: Duration::from_secs(2),
                    allowed_hosts: None,
                },
            )
            .await
            .expect("Execution failed");

        assert_eq!(result, "https://new-api.example.com:new_secret_456");
    }

    #[tokio::test]
    async fn test_cache() {
        let js_code = r#"
            function test_all() {
                const cached = this.orama.cache.get("counter");
                const count = cached ? cached + 1 : 1;
                this.orama.cache.set("counter", count);
                
                return `count: ${count}`;
            }
            export default { test_all };
        "#
        .to_string();

        let pool = JSPoolExecutor::<(), String>::new(
            js_code,
            10, // Multiple executor to test cache persistence across the pool
            None,
            None,
            None,
            Duration::from_secs(2),
            false,
            "test_all".to_string(),
        )
        .await
        .expect("Failed to create JSPoolExecutor");

        // To test that the cache is shared
        for i in 1..20 {
            let result = pool
                .exec(
                    (),
                    None,
                    ExecOption {
                        timeout: Duration::from_secs(2),
                        allowed_hosts: None,
                    },
                )
                .await
                .expect("Execution failed");

            assert_eq!(result, format!("count: {i}"));
        }
    }

    #[tokio::test]
    async fn test_missing_kv_and_secrets() {
        let js_code = r#"
            function test_missing() {
                const endpoint = this.orama.kv.get("nonexistent");
                const key = this.orama.secret.get("also_nonexistent");
                return `${endpoint}:${key}`;
            }
            export default { test_missing };
        "#
        .to_string();

        let pool = JSPoolExecutor::<(), String>::new(
            js_code,
            1,
            None, // No KV
            None, // No Secrets
            None,
            Duration::from_secs(2),
            false,
            "test_missing".to_string(),
        )
        .await
        .expect("Failed to create JSPoolExecutor");

        let result = pool
            .exec(
                (),
                None,
                ExecOption {
                    timeout: Duration::from_secs(2),
                    allowed_hosts: None,
                },
            )
            .await
            .expect("Execution failed");

        assert_eq!(result, "undefined:undefined");
    }
}
