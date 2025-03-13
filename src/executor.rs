use deno_core::error::CoreError;
use deno_core::{ModuleCodeString, ModuleSpecifier};
use thiserror::Error;
use tracing::{debug, trace};

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use deno_web::BlobStore;

use crate::orama_extension::{orama_extension, ChannelStorage};
use crate::permission::{CustomPermissions, DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING};

deno_core::extension!(deno_telemetry, esm = ["telemetry.ts", "util.ts"],);

pub static RUNTIME_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

const MAIN_IMPORT_MODULE_NAME: &str = "file:/main";
const GLOBAL_VARIABLE_NAME: &str = "__result";

#[derive(Error, Debug)]
pub enum JSExecutorError {
    #[error("Cannot start JSExecutor: {0}")]
    InitializationError(deno_core::error::CoreError),
    #[error("Bad JS code: {0}")]
    BadCode(deno_core::error::CoreError),
    #[error("A JS error is thrown: {0}")]
    ErrorThrown(deno_core::error::JsError),
    #[error("Unknown execution error: {0}")]
    UnknownExecutionError(deno_core::error::CoreError),
    #[error("The JS initialization took too long")]
    InitTimeout,
    #[error("The script does not provide an export named 'default'. Use 'export default {{ ... }}' to export the function")]
    NoDefaultExport,
    #[error("The script has a default export but it is not an object")]
    DefaultExportIsNotAnObject,
    #[error("The script has a default export but it does not provide an export named '{0}'")]
    NoExportedFunction(String),
    #[error("The script has a default export with the correct name, but type is wrong. Expected a function")]
    ExportedElementNotAFunction,
    #[error("The script took too long to execute")]
    ExecTimeout,
    #[error("Unable to deserialize the result: {0}")]
    DeserializationError(#[from] deno_core::serde_v8::Error),
    #[error("Domain not allowed: {0}")]
    DomainDenied(deno_core::error::JsError),
}

#[derive(Debug, Clone)]
pub struct JSExecutorConfig {
    pub allowed_hosts: Vec<String>,
    pub max_startup_time: Duration,
    pub max_execution_time: Duration,
    pub function_name: String,
    pub is_async: bool,
}

pub struct JSExecutor<Input: IntoFunctionParameters, Output: serde::de::DeserializeOwned> {
    js_runtime: deno_core::JsRuntime,
    max_execution_time: Duration,
    exec_count: usize,
    function_name: String,
    phantom: std::marker::PhantomData<(Input, Output)>,
    is_async: bool,
}

impl<Input: IntoFunctionParameters, Output: serde::de::DeserializeOwned> Debug
    for JSExecutor<Input, Output>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSExecutorConfig")
            .field("js_runtime", &"...")
            .field("max_execution_time", &self.max_execution_time)
            .field("exec_count", &self.exec_count)
            .field("function_name", &self.function_name)
            .field("is_async", &self.is_async)
            .finish()
    }
}

impl<Input: IntoFunctionParameters, Output: serde::de::DeserializeOwned + 'static>
    JSExecutor<Input, Output>
{
    pub async fn try_new<Code: Into<ModuleCodeString>>(
        config: JSExecutorConfig,
        code: Code,
    ) -> Result<Self, JSExecutorError> {
        let blob_store = BlobStore::default();
        let blob_store = Arc::new(blob_store);
        let code = code.into();

        let mut js_runtime = deno_core::JsRuntime::try_new(deno_core::RuntimeOptions {
            extensions: vec![
                deno_telemetry::init_ops(),
                deno_webidl::deno_webidl::init_ops(), // deno_url & deno_web
                deno_url::deno_url::init_ops(),       // deno_web
                deno_console::deno_console::init_ops(), // deno_web
                deno_web::deno_web::init_ops::<CustomPermissions>(blob_store, None),
                deno_net::deno_net::init_ops::<CustomPermissions>(None, None), // deno_web
                deno_fetch::deno_fetch::init_ops::<CustomPermissions>(
                    deno_fetch::Options::default(),
                ),
                orama_extension::init_ops(
                    CustomPermissions {
                        allowed_hosts: config.allowed_hosts,
                    },
                    ChannelStorage::<Output> { handler: None },
                ),
            ],
            startup_snapshot: Some(RUNTIME_SNAPSHOT),
            ..Default::default()
        })
        .map_err(JSExecutorError::InitializationError)?;

        // This never fails
        let m = ModuleSpecifier::parse(MAIN_IMPORT_MODULE_NAME).unwrap();
        let mod_id = js_runtime
            .load_main_es_module_from_code(&m, code)
            .await
            .map_err(JSExecutorError::BadCode)?;
        let result = js_runtime.mod_evaluate(mod_id);

        let timeout_result = tokio::time::timeout(
            // We allow async top level stuff, but it shouldn't take too long
            config.max_startup_time,
            js_runtime.run_event_loop(Default::default()),
        )
        .await;

        match timeout_result {
            Err(_) => {
                return Err(JSExecutorError::InitTimeout);
            }
            Ok(Err(e)) => {
                return Err(JSExecutorError::BadCode(e));
            }
            Ok(Ok(_)) => {}
        }

        result.await.map_err(JSExecutorError::BadCode)?;

        // test is the module has an export default
        Self::check_default_export(&mut js_runtime, &config.function_name).await?;

        Ok(Self {
            js_runtime,
            max_execution_time: config.max_execution_time,
            exec_count: 1,
            function_name: config.function_name,
            phantom: std::marker::PhantomData,
            is_async: config.is_async,
        })
    }

    async fn check_default_export(
        js_runtime: &mut deno_core::JsRuntime,
        function_name: &String,
    ) -> Result<(), JSExecutorError> {
        // This cannot fail
        let specifier = ModuleSpecifier::parse("file:/0").unwrap();
        let code = format!(
            r#"
import main from "{MAIN_IMPORT_MODULE_NAME}";
globalThis.{GLOBAL_VARIABLE_NAME} = 0;
if (typeof main !== 'object') {{
globalThis.{GLOBAL_VARIABLE_NAME} = 1;
}} else if (!main.{function_name}) {{
globalThis.{GLOBAL_VARIABLE_NAME} = 2;
}} else if (typeof main.{function_name} !== 'function') {{
globalThis.{GLOBAL_VARIABLE_NAME} = 3;
}}
"#,
        );

        trace!("Checking default export");
        Self::inner_exec(
            js_runtime,
            &specifier,
            code,
            // The above code should be faster
            Duration::from_secs(1),
            |e| match &e {
                CoreError::Js(error)
                    if error
                        .exception_message
                        .contains("does not provide an export named 'default'") =>
                {
                    JSExecutorError::NoDefaultExport
                }
                _ => JSExecutorError::BadCode(e),
            },
            |err| {
                // This should never happen: the code is hard coded
                JSExecutorError::InitializationError(err)
            },
            JSExecutorError::InitTimeout,
        )
        .await?;

        let output = Self::get_variable_value::<u8>(js_runtime, GLOBAL_VARIABLE_NAME)
            .map_err(JSExecutorError::DeserializationError)?;

        match output {
            // OK
            0 => {}
            1 => return Err(JSExecutorError::DefaultExportIsNotAnObject),
            2 => return Err(JSExecutorError::NoExportedFunction(function_name.clone())),
            3 => return Err(JSExecutorError::ExportedElementNotAFunction),
            // Unreachable by construction (see above code)
            _ => unreachable!(),
        }
        trace!("Default export check passed");

        Ok(())
    }

    pub async fn exec(&mut self, params: Input) -> Result<Output, JSExecutorError> {
        self.exec_count += 1;
        // Never fails because the format is correct
        let specifier = ModuleSpecifier::parse(&format!("file:/{}", self.exec_count)).unwrap();

        let await_keyword = if self.is_async { "await" } else { "" };
        let params = params.into_function_parameter().0;
        let code = format!(
            r#"
import main from "{MAIN_IMPORT_MODULE_NAME}";
globalThis.{GLOBAL_VARIABLE_NAME} = {await_keyword} main.{}(...{params:?});
"#,
            &self.function_name
        );

        debug!("Executing code");
        Self::inner_exec(
            &mut self.js_runtime,
            &specifier,
            code,
            self.max_execution_time,
            JSExecutorError::BadCode,
            |err| match err {
                CoreError::Js(error)
                // This check is not perfect, but it is good enough
                    if error
                        .exception_message
                        .contains(DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING) && error.exception_message.contains("which cannot be granted in this environment") =>
                {
                    JSExecutorError::DomainDenied(error)
                }
                CoreError::Js(error) => JSExecutorError::ErrorThrown(error),
                _ => JSExecutorError::UnknownExecutionError(err),
            },
            JSExecutorError::ExecTimeout,
        )
        .await?;

        debug!("Getting result");
        let output = Self::get_variable_value::<Output>(&mut self.js_runtime, GLOBAL_VARIABLE_NAME)
            .map_err(JSExecutorError::DeserializationError)?;

        debug!("done");
        Ok(output)
    }

    fn update_channel_storage(&mut self, handler: Option<Box<dyn FnMut(Output) + 'static>>) {
        let rc_state = self.js_runtime.op_state();
        let mut rc_state_ref = rc_state.borrow_mut();
        let state = &mut *rc_state_ref;
        state.put(ChannelStorage { handler });
        drop(rc_state_ref);
        drop(rc_state);
    }

    /// Attention: the handler is called sync with the JS code
    pub async fn exec_stream<F>(&mut self, params: Input, handler: F) -> Result<(), JSExecutorError>
    where
        F: FnMut(Output) + 'static,
    {
        self.exec_count += 1;
        // Never fails because the format is correct
        let specifier = ModuleSpecifier::parse(&format!("file:/{}", self.exec_count)).unwrap();

        // Set the handler
        self.update_channel_storage(Some(Box::new(handler)));

        let params = params.into_function_parameter().0;
        let code = format!(
            r#"
import main from "{MAIN_IMPORT_MODULE_NAME}";
const generator = main.{}(...{params:?});
for await (const value of generator) {{
    Deno.core.ops.send_data_to_channel(value);
}}
"#,
            self.function_name
        );

        debug!("Executing code");
        Self::inner_exec(
            &mut self.js_runtime,
            &specifier,
            code,
            self.max_execution_time,
            JSExecutorError::BadCode,
            |err| match err {
                CoreError::Js(error)
                // This check is not perfect, but it is good enough
                    if error
                        .exception_message
                        .contains(DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING) && error.exception_message.contains("which cannot be granted in this environment") =>
                {
                    JSExecutorError::DomainDenied(error)
                }
                CoreError::Js(error) => JSExecutorError::ErrorThrown(error),
                _ => JSExecutorError::UnknownExecutionError(err),
            },
            JSExecutorError::ExecTimeout,
        )
        .await?;

        // This is needed to drop the handler correctly
        // So, if the handler holds a reference to something, it can be dropped
        // An example is the mpsc::Sender: the blow code is needed to drop the sender
        self.update_channel_storage(None);

        Ok(())
    }

    async fn inner_exec<F, F2>(
        js_runtime: &mut deno_core::JsRuntime,
        specifier: &ModuleSpecifier,
        code: String,
        timeout_duration: Duration,
        on_load_error: F,
        runtime_error: F2,
        error_on_timeout: JSExecutorError,
    ) -> Result<(), JSExecutorError>
    where
        F: FnOnce(deno_core::error::CoreError) -> JSExecutorError,
        F2: FnOnce(deno_core::error::CoreError) -> JSExecutorError,
    {
        let mod_id = js_runtime
            .load_side_es_module_from_code(specifier, code)
            .await
            .map_err(on_load_error)?;
        let result = js_runtime.mod_evaluate(mod_id);

        let timeout_result = tokio::time::timeout(
            timeout_duration,
            js_runtime.run_event_loop(Default::default()),
        )
        .await;

        match timeout_result {
            Err(_) => {
                return Err(error_on_timeout);
            }
            Ok(Err(e)) => {
                return Err(runtime_error(e));
            }
            Ok(Ok(_)) => {}
        }

        result.await.map_err(JSExecutorError::BadCode)?;

        Ok(())
    }

    fn get_variable_value<VariableType: serde::de::DeserializeOwned>(
        js_runtime: &mut deno_core::JsRuntime,
        variable_name: &str,
    ) -> Result<VariableType, deno_core::serde_v8::Error> {
        let mut scope: deno_core::v8::HandleScope<'_> = js_runtime.handle_scope();
        let context = scope.get_current_context();
        let global = context.global(&mut scope);
        // The output is always there because it is hard coded
        let key = deno_core::v8::String::new(&mut scope, variable_name).unwrap();
        // The value is always there because it is hard coded
        let value = global.get(&mut scope, key.into()).unwrap();
        deno_core::serde_v8::from_v8(&mut scope, value)
    }
}

pub struct FunctionParameters(Vec<serde_json::Value>);

impl Default for FunctionParameters {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionParameters {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn push<T: IntoFunctionParameters>(&mut self, value: T) {
        self.0.extend(value.into_function_parameter().0);
    }

    pub fn into_inner(self) -> Vec<serde_json::Value> {
        self.0
    }
}

pub trait IntoFunctionParameters {
    fn into_function_parameter(self) -> FunctionParameters;
}

impl<T> IntoFunctionParameters for T
where
    T: Into<serde_json::Value>,
{
    fn into_function_parameter(self) -> FunctionParameters {
        let mut v: serde_json::Value = self.into();
        if v.is_array() {
            // Avoid cloning the array
            let v = v.as_array_mut().unwrap().drain(..).collect();
            FunctionParameters(v)
        } else {
            FunctionParameters(vec![v])
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::sleep;

    use super::*;

    #[tokio::test]
    async fn test_fixed_sync() {
        let config = JSExecutorConfig {
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: false,
        };

        let mut executor = JSExecutor::try_new(
            config,
            r#"
function zz(a, b, c) {
    return a + b + c;
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();
        let output: String = executor.exec("pippo".to_string()).await.unwrap();

        assert_eq!(output, "pippoundefinedundefined");
    }

    #[tokio::test]
    async fn test_fixed_json_compatibility() {
        let config = JSExecutorConfig {
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: false,
        };

        let mut executor = JSExecutor::try_new(
            config,
            r#"
function zz(a, b, c) {
    return a + b + c;
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();
        let output: i32 = executor.exec(vec![1, 2, 3]).await.unwrap();

        assert_eq!(output, 6);
    }

    #[tokio::test]
    async fn test_fixed_async() {
        let config = JSExecutorConfig {
            allowed_hosts: vec!["jsonplaceholder.typicode.com".to_string()],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let mut executor = JSExecutor::try_new(
            config,
            r#"
async function zz(id) {
    const response = await fetch(`https://jsonplaceholder.typicode.com/todos/${id}`);
    const json = await response.json();
    return json;
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();
        let output: Todo = executor.exec(vec![1]).await.unwrap();

        assert_eq!(
            output,
            Todo {
                user_id: 1,
                id: 1,
                title: "delectus aut autem".to_string(),
                completed: false,
            }
        );
    }

    #[tokio::test]
    async fn test_fixed_async_permission_denied() {
        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let mut executor = JSExecutor::try_new(
            config,
            r#"
async function zz(id) {
    const response = await fetch(`https://jsonplaceholder.typicode.com/todos/${id}`);
    const json = await response.json();
    return json;
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();
        let output: Result<Todo, JSExecutorError> = executor.exec(vec![1]).await;
        let err = output.unwrap_err();
        assert!(matches!(err, JSExecutorError::DomainDenied(_)));
    }

    #[tokio::test]
    async fn test_fixed_async_throw_error() {
        let config = JSExecutorConfig {
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let mut executor = JSExecutor::try_new(
            config,
            r#"
async function zz(id) {
    throw new Error("pippo");
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();
        let output: Result<Todo, JSExecutorError> = executor.exec(vec![1]).await;
        let err = output.unwrap_err();
        assert!(matches!(err, JSExecutorError::ErrorThrown(_)));
    }

    #[tokio::test]
    async fn test_fixed_async_error_no_default_export() {
        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let executor = JSExecutor::<Vec<String>, i32>::try_new(
            config,
            r#"
export const foo = 42;
"#
            .to_string(),
        )
        .await;
        let err = executor.unwrap_err();
        assert!(matches!(err, JSExecutorError::NoDefaultExport));

        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let executor = JSExecutor::<Vec<String>, i32>::try_new(
            config,
            r#"
export default 42;
"#
            .to_string(),
        )
        .await;
        let err = executor.unwrap_err();
        assert!(matches!(err, JSExecutorError::DefaultExportIsNotAnObject));

        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let executor = JSExecutor::<Vec<String>, i32>::try_new(
            config,
            r#"
export default {};
"#
            .to_string(),
        )
        .await;
        let err = executor.unwrap_err();
        assert!(matches!(err, JSExecutorError::NoExportedFunction(_)));

        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let executor = JSExecutor::<Vec<String>, i32>::try_new(
            config,
            r#"
export default { myFunction: 42 };
"#
            .to_string(),
        )
        .await;
        let err = executor.unwrap_err();
        assert!(matches!(err, JSExecutorError::ExportedElementNotAFunction));
    }

    #[tokio::test]
    async fn test_fixed_async_generator_async() {
        let config = JSExecutorConfig {
            // Permission denied
            allowed_hosts: vec![],
            max_execution_time: Duration::from_secs(2),
            max_startup_time: Duration::from_millis(300),
            function_name: "myFunction".to_string(),
            is_async: true,
        };

        let mut executor: JSExecutor<Vec<i32>, String> = JSExecutor::try_new(
            config,
            r#"
async function* zz(id) {
    yield `${id++}`;
    yield `${id++}`;
    yield `${id++}`;
}
export default { myFunction: zz };
"#
            .to_string(),
        )
        .await
        .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        executor
            .exec_stream(vec![1], move |v| {
                tx.send(v).unwrap();
            })
            .await
            .unwrap();

        assert_eq!(
            rx.recv().await.unwrap(),
            serde_json::Value::String("1".to_string())
        );
        assert_eq!(
            rx.recv().await.unwrap(),
            serde_json::Value::String("2".to_string())
        );
        assert_eq!(
            rx.recv().await.unwrap(),
            serde_json::Value::String("3".to_string())
        );

        sleep(Duration::from_millis(200)).await;

        assert!(rx.is_closed());
    }

    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct Todo {
        #[serde(rename = "userId")]
        user_id: u32,
        id: u32,
        title: String,
        completed: bool,
    }
}
