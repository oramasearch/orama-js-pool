use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use deno_core::ModuleCodeString;
use tokio::sync::RwLock;

use crate::orama_extension::OutputChannel;
use crate::runner::ExecOption;
use crate::{executor::JSExecutor, JSRunnerError, TryIntoFunctionParameters};

pub struct JSPoolExecutor<Input, Output> {
    executors: Arc<Vec<RwLock<JSExecutor<Input, Output>>>>,
    index: AtomicUsize,
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
    pub async fn new<Code: Into<ModuleCodeString> + Send + Clone + 'static>(
        code: Code,
        instances: usize,
        allowed_hosts_on_init: Option<Vec<String>>,
        timeout_on_init: std::time::Duration,
        is_async: bool,
        function_name: String,
    ) -> Result<Self, JSRunnerError> {
        let mut executors = Vec::with_capacity(instances);
        for _ in 0..(instances) {
            let executor = JSExecutor::try_new(
                code.clone(),
                allowed_hosts_on_init.clone(),
                timeout_on_init,
                is_async,
                function_name.clone(),
            )
            .await?;
            let executor = RwLock::new(executor);
            executors.push(executor);
        }

        let executors = Arc::new(executors);

        Ok(Self {
            executors,
            index: AtomicUsize::new(0),
        })
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
        "#.to_string();

        let pool = JSPoolExecutor::<TestInput, i32>::new(
            js_code,
            2, // two executors
            None,
            Duration::from_secs(2),
            false,
            "addOne".to_string(),
        )
        .await
        .expect("Failed to create JSPoolExecutor");

        let result = pool
            .exec(TestInput(41), None, ExecOption { timeout: Duration::from_secs(2), allowed_hosts: None })
            .await
            .expect("Execution failed");

        assert_eq!(result, 42);
    }
}
