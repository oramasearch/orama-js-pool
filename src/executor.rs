use std::{sync::Arc, time::Duration};

use deno_core::{FastString, ModuleCodeString};

use crate::{
    orama_extension::{OutputChannel, SharedCache, SharedKV, SharedSecrets},
    runner::{load_code, ExecOption, JSRunnerError, LoadedJsFunction},
    TryIntoFunctionParameters,
};
use std::collections::HashMap;

pub struct JSExecutor<Input, Output> {
    loaded_function: LoadedJsFunction<Input, Output>,
    code: FastString,
    function_name: String,
    is_async: bool,
    allowed_hosts_on_init: Option<Vec<String>>,
    timeout_on_init: Duration,
    shared_cache: SharedCache,
    shared_kv: SharedKV,
    shared_secrets: SharedSecrets,
}

impl<Input: TryIntoFunctionParameters, Output: serde::de::DeserializeOwned + 'static>
    JSExecutor<Input, Output>
{
    #[allow(clippy::too_many_arguments)]
    pub async fn try_new<Code: Into<ModuleCodeString> + Send + 'static>(
        code: Code,
        allowed_hosts_on_init: Option<Vec<String>>,
        timeout_on_init: Duration,
        is_async: bool,
        function_name: String,
        shared_cache: SharedCache,
        shared_kv: SharedKV,
        shared_secrets: SharedSecrets,
    ) -> Result<Self, JSRunnerError> {
        let code = code.into();

        let (copy1, copy2) = code.into_cheap_copy();
        let loaded_code = load_code::<FastString, Input, Output>(
            copy1,
            allowed_hosts_on_init.clone(),
            timeout_on_init,
            shared_cache.clone(),
            shared_kv.clone(),
            shared_secrets.clone(),
        )
        .await?;

        let loaded_function = loaded_code
            .into_function(is_async, function_name.clone())
            .await?;

        Ok(Self {
            loaded_function,
            code: copy2,
            function_name,
            is_async,
            allowed_hosts_on_init,
            timeout_on_init,
            shared_cache,
            shared_kv,
            shared_secrets,
        })
    }

    /// Create a builder for JSExecutor
    pub fn builder<Code>(
        code: Code,
        function_name: impl Into<String>,
    ) -> JSExecutorBuilder<Input, Output, Code>
    where
        Code: Into<ModuleCodeString> + Send + 'static,
    {
        JSExecutorBuilder::new(code, function_name)
    }

    pub async fn exec(
        &mut self,
        params: Input,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        option: ExecOption,
    ) -> Result<Output, JSRunnerError> {
        if !self.loaded_function.is_alive() {
            let code = self.code.try_clone().expect("Always clonable");

            let loaded_code = load_code::<FastString, Input, Output>(
                code,
                self.allowed_hosts_on_init.clone(),
                self.timeout_on_init,
                self.shared_cache.clone(),
                self.shared_kv.clone(),
                self.shared_secrets.clone(),
            )
            .await?;

            let loaded_function = loaded_code
                .into_function(self.is_async, self.function_name.clone())
                .await?;

            self.loaded_function = loaded_function;
        }
        self.loaded_function
            .exec(params, stdout_sender, option)
            .await
    }
}

pub struct JSExecutorBuilder<Input, Output, Code> {
    code: Code,
    function_name: String,
    kv: Option<HashMap<String, String>>,
    secrets: Option<HashMap<String, String>>,
    allowed_hosts_on_init: Option<Vec<String>>,
    timeout_on_init: Duration,
    is_async: bool,
    _phantom: std::marker::PhantomData<(Input, Output)>,
}

impl<Input, Output, Code> JSExecutorBuilder<Input, Output, Code>
where
    Input: TryIntoFunctionParameters,
    Output: serde::de::DeserializeOwned + 'static,
    Code: Into<ModuleCodeString> + Send + 'static,
{
    pub fn new(code: Code, function_name: impl Into<String>) -> Self {
        Self {
            code,
            function_name: function_name.into(),
            kv: None,
            secrets: None,
            allowed_hosts_on_init: Some(vec![]),
            timeout_on_init: Duration::from_secs(30),
            is_async: false,
            _phantom: std::marker::PhantomData,
        }
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

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout_on_init = timeout;
        self
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    pub async fn build(self) -> Result<JSExecutor<Input, Output>, JSRunnerError> {
        let shared_cache = SharedCache::new();
        let shared_kv = match self.kv {
            Some(map) => SharedKV::from_map(map),
            None => SharedKV::new(),
        };
        let shared_secrets = match self.secrets {
            Some(map) => SharedSecrets::from_map(map),
            None => SharedSecrets::new(),
        };

        JSExecutor::try_new(
            self.code,
            self.allowed_hosts_on_init,
            self.timeout_on_init,
            self.is_async,
            self.function_name,
            shared_cache,
            shared_kv,
            shared_secrets,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_function_async_timeout_reuse() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut executor: JSExecutor<u32, String> = JSExecutor::try_new(
            r#"
async function foo(timeout) {
    await new Promise(r => setTimeout(r, timeout)) // 2 sec
    return 'DONE'
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            true,
            "foo".to_string(),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        for _ in 0..50 {
            let ret = executor
                .exec(
                    100,
                    None,
                    ExecOption {
                        allowed_hosts: None,
                        timeout: Duration::from_millis(10),
                    },
                )
                .await;
            let err = ret.err().unwrap();
            assert!(matches!(err, JSRunnerError::ExecTimeout));

            let ret = executor
                .exec(
                    10,
                    None,
                    ExecOption {
                        allowed_hosts: None,
                        timeout: Duration::from_millis(500),
                    },
                )
                .await
                .unwrap();
            assert_eq!(ret, "DONE".to_string());
        }
    }
}
