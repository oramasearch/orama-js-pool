use std::{sync::Arc, time::Duration};

use deno_core::{FastString, ModuleCodeString};

use crate::{
    orama_extension::OutputChannel,
    runner::{load_code, ExecOption, JSRunnerError, LoadedJsFunction},
    TryIntoFunctionParameters,
};

pub struct JSExecutor<Input, Output> {
    loaded_function: LoadedJsFunction<Input, Output>,
    code: FastString,
    function_name: String,
    is_async: bool,
    allowed_hosts_on_init: Option<Vec<String>>,
    timeout_on_init: Duration,
}

impl<Input: TryIntoFunctionParameters, Output: serde::de::DeserializeOwned + 'static>
    JSExecutor<Input, Output>
{
    pub async fn try_new<Code: Into<ModuleCodeString> + Send + 'static>(
        code: Code,
        allowed_hosts_on_init: Option<Vec<String>>,
        timeout_on_init: Duration,
        is_async: bool,
        function_name: String,
    ) -> Result<Self, JSRunnerError> {
        let code = code.into();

        let (copy1, copy2) = code.into_cheap_copy();
        let loaded_code = load_code::<FastString, Input, Output>(
            copy1,
            allowed_hosts_on_init.clone(),
            timeout_on_init,
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
        })
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
