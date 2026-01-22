use std::{marker::PhantomData, sync::Arc, thread::JoinHandle, time::Duration};

use deno_core::{
    error::CoreError, v8::IsolateHandle, ModuleCodeString, ModuleSpecifier, PollEventLoopOptions,
};
use deno_web::BlobStore;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::{runtime::Builder, task::LocalSet};
use tracing::{debug, info, trace, warn};

use crate::{
    orama_extension::{
        orama_extension, ChannelStorage, OutputChannel, SharedCache, SharedKV, SharedSecrets,
        StdoutHandler, StdoutHandlerFn,
    },
    parameters::TryIntoFunctionParameters,
    permission::CustomPermissions,
};

deno_core::extension!(deno_telemetry, esm = ["telemetry.ts", "util.ts"],);

pub static RUNTIME_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

const MAIN_IMPORT_MODULE_NAME: &str = "file:/main";
const GLOBAL_VARIABLE_NAME: &str = "__result";

#[derive(Error, Debug)]
pub enum JSRunnerError {
    #[error("Cannot start JSExecutor: {0}")]
    InitializationError(Box<deno_core::error::CoreError>),
    // #[error("Bad JS code: {0}")]
    // BadCode(deno_core::error::CoreError),
    #[error("A JS error is thrown: {0}")]
    ErrorThrown(Box<deno_core::error::JsError>),
    #[error("Unknown execution error: {0}")]
    UnknownExecutionError(Box<deno_core::error::CoreError>),
    #[error("The JS initialization took too long")]
    InitTimeout,
    // #[error("The script does not provide an export named 'default'. Use 'export default {{ ... }}' to export the function")]
    // NoDefaultExport,
    #[error("The script has a default export but it is not an object")]
    DefaultExportIsNotAnObject,
    #[error("The script has a default export but it does not provide an export named '{0}'")]
    NoExportedFunction(String),
    #[error("The script has a default export with the correct name, but type is wrong. Expected a function")]
    ExportedElementNotAFunction,
    #[error("The script took too long to execute")]
    ExecTimeout,
    // #[error("Unable to deserialize the result: {0}")]
    // V8DeserializationError(#[from] deno_core::serde_v8::Error),
    // #[error("Domain not allowed: {0}")]
    // HTTPDomainDenied(deno_core::error::JsError),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("compilation error: {0}")]
    CompilationError(Box<deno_core::error::JsError>),

    #[error("code is required but was not provided")]
    MissingCode,

    #[error("function name is required but was not provided")]
    MissingFunctionName,

    #[error("unknown")]
    Unknown,
}

pub enum LoadedCodeEvent {
    Stop,
    CheckFunction(String, tokio::sync::oneshot::Sender<u8>),
    ExecFunction {
        id: u64,
        is_async: bool,
        function_name: String,
        input_params: String,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        allowed_hosts: Option<Vec<String>>,
        sender: tokio::sync::oneshot::Sender<Result<serde_json::Value, JSRunnerError>>,
    },
}

pub struct LoadedJsModule<Input, Output> {
    handler: IsolateHandle,
    join_handler: JoinHandle<()>,
    sender: tokio::sync::mpsc::Sender<LoadedCodeEvent>,
    p: PhantomData<(Input, Output)>,
}

impl<Input, Output> LoadedJsModule<Input, Output> {
    pub async fn into_function(
        self,
        is_async: bool,
        function_name: String,
    ) -> Result<LoadedJsFunction<Input, Output>, JSRunnerError> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<u8>();

        self.sender
            .send(LoadedCodeEvent::CheckFunction(
                function_name.clone(),
                sender,
            ))
            .await
            .unwrap();
        let output = receiver.await.unwrap();

        match output {
            // OK
            0 => {}
            1 => return Err(JSRunnerError::DefaultExportIsNotAnObject),
            2 => return Err(JSRunnerError::NoExportedFunction(function_name)),
            3 => return Err(JSRunnerError::ExportedElementNotAFunction),
            // Unreachable by construction (see above code)
            _ => unreachable!(),
        };

        Ok(LoadedJsFunction {
            function_name,
            is_async,
            timed_out: false,
            join_handler: self.join_handler,
            handler: self.handler.clone(),
            sender: self.sender.clone(),
            exec_count: 1, // 1 because we used 0 to check the function
            _p: PhantomData,
        })
    }
}

pub struct ExecOption {
    pub timeout: Duration,
    pub allowed_hosts: Option<Vec<String>>,
}

pub struct LoadedJsFunction<Input, Output> {
    sender: tokio::sync::mpsc::Sender<LoadedCodeEvent>,
    handler: IsolateHandle,
    join_handler: JoinHandle<()>,
    is_async: bool,
    function_name: String,
    exec_count: u64,
    timed_out: bool,
    _p: PhantomData<(Input, Output)>,
}

impl<Input, Output> Drop for LoadedJsFunction<Input, Output> {
    fn drop(&mut self) {
        self.handler.terminate_execution();
    }
}

impl<Input: TryIntoFunctionParameters, Output: DeserializeOwned + 'static>
    LoadedJsFunction<Input, Output>
{
    pub fn is_alive(&self) -> bool {
        !(self.timed_out || self.join_handler.is_finished())
    }

    pub async fn exec(
        &mut self,
        params: Input,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        option: ExecOption,
    ) -> Result<Output, JSRunnerError> {
        if !self.is_alive() {
            unreachable!("NOOOPE");
        }

        let id = self.exec_count;
        self.exec_count += 1;

        let ExecOption {
            allowed_hosts,
            timeout,
        } = option;

        let (sender, receiver) =
            tokio::sync::oneshot::channel::<Result<serde_json::Value, JSRunnerError>>();

        let params = params.try_into_function_parameter()?;
        let params = params.0;

        let input_params = serde_json::to_string(&params).map_err(JSRunnerError::SerdeError)?;

        info!("Ask to exec");
        self.sender
            .send(LoadedCodeEvent::ExecFunction {
                id,
                is_async: self.is_async,
                function_name: self.function_name.clone(),
                input_params,
                stdout_sender,
                allowed_hosts,
                sender,
            })
            .await
            .unwrap();

        let output = tokio::time::timeout(timeout, receiver).await;

        let output = match output {
            Err(_) => {
                self.timed_out = true;
                self.handler.terminate_execution();
                // timeout
                return Err(JSRunnerError::ExecTimeout);
            }
            Ok(Err(_)) => {
                unreachable!("aaa");
            }
            Ok(Ok(Ok(t))) => t,
            Ok(Ok(Err(e))) => return Err(e),
        };

        info!("Executed");

        let output: Output = serde_json::from_value(output).map_err(JSRunnerError::SerdeError)?;

        Ok(output)
    }
}

pub async fn load_code<
    Code: Into<ModuleCodeString> + Send + 'static,
    Input: TryIntoFunctionParameters,
    Output: serde::de::DeserializeOwned + 'static,
>(
    code: Code,
    allowed_hosts: Option<Vec<String>>,
    timeout: Duration,
    shared_cache: SharedCache,
    shared_kv: SharedKV,
    shared_secrets: SharedSecrets,
) -> Result<LoadedJsModule<Input, Output>, JSRunnerError> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<LoadedCodeEvent>(1);
    let (init_sender1, init_receiver1) =
        tokio::sync::oneshot::channel::<Result<IsolateHandle, CoreError>>();
    let (init_sender2, init_receiver2) = tokio::sync::oneshot::channel::<Result<(), CoreError>>();

    let thread_id = std::thread::spawn(|| {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        let local = LocalSet::new();
        local.spawn_local(async move {
            let blob_store = BlobStore::default();
            let blob_store = Arc::new(blob_store);
            let code = code.into();

            let js_runtime = deno_core::JsRuntime::try_new(deno_core::RuntimeOptions {
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
                        CustomPermissions { allowed_hosts },
                        ChannelStorage::<Output> {
                            stream_handler: None,
                        },
                        StdoutHandler(None),
                        shared_cache,
                        shared_kv,
                        shared_secrets,
                    ),
                ],
                startup_snapshot: Some(RUNTIME_SNAPSHOT),
                ..Default::default()
            });

            let mut js_runtime = match js_runtime {
                Ok(js_runtime) => js_runtime,
                Err(e) => {
                    warn!("Cannot instantiate JsRuntime");
                    init_sender1.send(Err(e)).expect("Cannot send Err init 1");
                    return;
                }
            };

            let handler = js_runtime.handle_scope().thread_safe_handle();
            init_sender1
                .send(Ok(handler))
                .expect("Cannot send thread_safe_handle init 1");

            info!("Loading ES main module");

            // This never fails
            let m = ModuleSpecifier::parse(MAIN_IMPORT_MODULE_NAME).unwrap();
            let mod_id = js_runtime.load_main_es_module_from_code(&m, code).await;

            let mod_id = match mod_id {
                Ok(mod_id) => mod_id,
                Err(e) => {
                    warn!("Cannot load main ES module");
                    init_sender2.send(Err(e)).expect("C");
                    return;
                }
            };

            info!("Eval ES main module");

            let eval = js_runtime.mod_evaluate(mod_id);

            let output = js_runtime
                .run_event_loop(PollEventLoopOptions::default())
                .await;
            if let Err(e) = output {
                warn!("Error in evaluation main module");
                let _ = init_sender2.send(Err(e));
                return;
            }

            if let Err(e) = eval.await {
                warn!("Cannot evaluate mod {mod_id}");
                let _ = init_sender2.send(Err(e));
                return;
            }

            init_sender2
                .send(Ok(()))
                .expect("Cannot send mod_evaluate ok init 2 ");

            info!("Waiting for new events...");
            while let Some(ev) = receiver.recv().await {
                debug!("Received event...");
                match ev {
                    LoadedCodeEvent::Stop => {
                        warn!("Stopping loop due to received command");
                        break;
                    }
                    LoadedCodeEvent::CheckFunction(function_name, sender) => {
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

                        let mod_id = match js_runtime
                            .load_side_es_module_from_code(&specifier, code)
                            .await
                        {
                            Ok(mod_id) => mod_id,
                            Err(CoreError::Js(e)) => {
                                // No default export at all. Threated as `1`
                                if e.name.as_ref() == Some(&"SyntaxError".to_string())
                                    && e.message.as_ref().is_some_and(|s| {
                                        s.contains("does not provide an export named 'default'")
                                    })
                                {
                                    let _ = sender.send(1);
                                    continue;
                                }
                                panic!("load_side_es_module_from_code Err JS {e:?}");
                            }
                            Err(e) => {
                                panic!("load_side_es_module_from_code {e:?}");
                            }
                        };

                        let eval = js_runtime.mod_evaluate(mod_id);

                        match js_runtime.run_event_loop(Default::default()).await {
                            Ok(_) => {}
                            Err(e) => {
                                panic!("run_event_loop {e:?}");
                            }
                        };

                        match eval.await {
                            Ok(_) => {}
                            Err(e) => {
                                panic!("eval: {e:?}");
                            }
                        };

                        let mut scope: deno_core::v8::HandleScope<'_> = js_runtime.handle_scope();
                        let context = scope.get_current_context();
                        let global = context.global(&mut scope);
                        // The output is always there because it is hard coded
                        let key =
                            deno_core::v8::String::new(&mut scope, GLOBAL_VARIABLE_NAME).unwrap();
                        // The value is always there because it is hard coded
                        let value = global.get(&mut scope, key.into()).unwrap();
                        let output: u8 = deno_core::serde_v8::from_v8(&mut scope, value).unwrap();
                        let _ = sender.send(output);
                    }
                    LoadedCodeEvent::ExecFunction {
                        id,
                        is_async,
                        function_name,
                        input_params,
                        stdout_sender,
                        allowed_hosts,
                        sender,
                    } => {
                        debug!("Overriding state");
                        update_inner_state(&mut js_runtime, stdout_sender, allowed_hosts);
                        debug!("State overridden");

                        // Never fails because the format is correct
                        let specifier = ModuleSpecifier::parse(&format!("file:/{id}")).unwrap();

                        let await_keyword = if is_async { "await" } else { "" };
                        let code = format!(
                            r#"
import main from "{MAIN_IMPORT_MODULE_NAME}";
const thisContext = {{
    orama: {{
        cache: {{
            get: (key) => Deno.core.ops.op_cache_get(key) ?? undefined,
            set: (key, value, options) => Deno.core.ops.op_cache_set(key, value, options?.ttl),
            delete: (key) => Deno.core.ops.op_cache_delete(key)
        }},
        kv: {{
            get: (key) => Deno.core.ops.op_kv_get(key) ?? undefined
        }},
        secret: {{
            get: (key) => Deno.core.ops.op_secret_get(key) ?? undefined
        }}
    }}
}};
globalThis.{GLOBAL_VARIABLE_NAME} = {await_keyword} main.{}.call(thisContext, ...{input_params});
                "#,
                            &function_name
                        );

                        info!("Running code");

                        let mod_id = match js_runtime
                            .load_side_es_module_from_code(&specifier, code)
                            .await
                        {
                            Ok(mod_id) => mod_id,
                            Err(e) => {
                                panic!("load_side_es_module_from_code {e:?}");
                            }
                        };
                        debug!("Evaluating code");

                        let eval = js_runtime.mod_evaluate(mod_id);

                        match js_runtime.run_event_loop(Default::default()).await {
                            Ok(_) => {}
                            Err(e) => {
                                match e {
                                    CoreError::Js(e) => {
                                        let _ = sender
                                            .send(Err(JSRunnerError::ErrorThrown(Box::new(e))));
                                    }
                                    _ => {
                                        let _ = sender.send(Err(
                                            JSRunnerError::UnknownExecutionError(Box::new(e)),
                                        ));
                                    }
                                };
                                return;
                            }
                        };

                        match eval.await {
                            Ok(_) => {}
                            Err(e) => {
                                panic!("eval Err {e:?}");
                            }
                        };

                        info!("Done");

                        let mut scope: deno_core::v8::HandleScope<'_> = js_runtime.handle_scope();
                        let context = scope.get_current_context();
                        let global = context.global(&mut scope);
                        // The output is always there because it is hard-coded
                        let key =
                            deno_core::v8::String::new(&mut scope, GLOBAL_VARIABLE_NAME).unwrap();
                        // The value is always there because it is hard-coded
                        let value = global.get(&mut scope, key.into()).unwrap();
                        let output: serde_json::Value =
                            deno_core::serde_v8::from_v8(&mut scope, value).unwrap();

                        trace!("Ouptut {:?}", output);

                        let _ = sender.send(Ok(output));
                    }
                };
            }

            info!("task is stopped")
        });

        rt.block_on(local);

        info!("thread is stopped")
    });

    let handler = init_receiver1.await.unwrap();
    let handler = match handler {
        Ok(handler) => handler,
        Err(e) => {
            warn!("{e:?}");
            return Err(JSRunnerError::InitTimeout);
        }
    };

    let output = tokio::time::timeout(timeout, init_receiver2).await;
    match output {
        Err(_) => {
            warn!("Start up time tooks too much time. Stop it");
            handler.terminate_execution();
            let _ = sender.send(LoadedCodeEvent::Stop).await;
            let _ = thread_id.join(); // ignore this error.
            return Err(JSRunnerError::InitTimeout);
        }
        Ok(Err(e)) => {
            panic!("RecvError {e:?}")
        }
        Ok(Ok(Err(e))) => {
            warn!("Error in start up");
            handler.terminate_execution();
            let _ = sender.send(LoadedCodeEvent::Stop).await;
            let _ = thread_id.join(); // ignore this error.

            match e {
                CoreError::Js(e) => {
                    if e.name.as_ref().is_some_and(|s| s == "SyntaxError") {
                        return Err(JSRunnerError::CompilationError(Box::new(e)));
                    }
                    return Err(JSRunnerError::InitializationError(Box::new(CoreError::Js(
                        e,
                    ))));
                }
                _ => return Err(JSRunnerError::InitializationError(Box::new(e))),
            };
        }
        Ok(Ok(Ok(_))) => {
            info!("Loaded");
        }
    };

    Ok(LoadedJsModule {
        handler,
        join_handler: thread_id,
        sender,
        p: PhantomData,
    })
}

fn update_inner_state(
    js_runtime: &mut deno_core::JsRuntime,
    stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
    allowed_hosts: Option<Vec<String>>,
) {
    let rc_state = js_runtime.op_state();
    let mut rc_state_ref = rc_state.borrow_mut();
    let state = &mut *rc_state_ref;
    let stdout_handler: Option<StdoutHandlerFn> = if let Some(stdout_sender) = stdout_sender {
        Some(Box::new(move |a: &str, b: OutputChannel| {
            match stdout_sender.send((b, a.to_string())) {
                Ok(_) => {}
                Err(e) => debug!("Cannot send  {e:?}"),
            };
        }))
    } else {
        None
    };
    state.put(StdoutHandler(stdout_handler));
    state.put(CustomPermissions { allowed_hosts });
    drop(rc_state_ref);
    drop(rc_state);
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[tokio::test]
    async fn test_basic() {
        let _ = tracing_subscriber::fmt::try_init();

        load_code::<String, String, String>(
            r#"
export function life() {
    return 42
}
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_while_1_on_load() {
        let _ = tracing_subscriber::fmt::try_init();

        let runner = load_code::<String, String, String>(
            r#"
while (1) {}

export function life() {
    return 42
}
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await;

        let err = runner.err().unwrap();
        assert!(matches!(err, JSRunnerError::InitTimeout));
    }

    #[tokio::test]
    async fn test_throw_on_init() {
        let _ = tracing_subscriber::fmt::try_init();

        let runner = load_code::<String, String, String>(
            r#"

throw new Error('KABOOM')

export function life() {
    return 42
}
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await;

        let err = runner.err().unwrap();
        let JSRunnerError::InitializationError(e) = err else {
            panic!("Not InitializationError");
        };
        assert!(e.print_with_cause().contains("KABOOM"));
    }

    #[tokio::test]
    async fn test_fetch_on_init() {
        let _ = tracing_subscriber::fmt::try_init();

        let runner = load_code::<String, String, String>(
            r#"

await fetch('http://localhost:3000');

export function life() {
    return 42
}
            "#
            .to_string(),
            Some(vec![]), // `localhost:3000` is not allowed
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await;

        let err = runner.err().unwrap();
        let JSRunnerError::InitializationError(e) = err else {
            panic!("Not InitializationError");
        };
        assert!(e.print_with_cause().contains("Domain not allowed"));
    }

    #[tokio::test]
    async fn test_compilation_error() {
        let _ = tracing_subscriber::fmt::try_init();

        let runner = load_code::<String, String, String>(
            r#"
export function life() {
            "#
            .to_string(),
            Some(vec![]), // `localhost:3000` is not allowed
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await;

        let err = runner.err().unwrap();
        let JSRunnerError::CompilationError(e) = err else {
            panic!("Not InitializationError");
        };
        assert!(format!("{e:?}").contains("SyntaxError"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_allowed_external_http_invocation_on_init() {
        let _ = tracing_subscriber::fmt::try_init();

        let addr = start_http_server().await;

        load_code::<String, String, String>(
            format!(
                r#"
await fetch('http://localhost:{}/');
            "#,
                addr.port()
            ),
            Some(vec![format!("localhost:{}", addr.port())]),
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_get_function_ok() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_function_no_export() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {}

// export default { } No export at all
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let function = loaded_code.into_function(false, "foo".to_string()).await;

        let err = function.err().unwrap();
        assert!(matches!(err, JSRunnerError::DefaultExportIsNotAnObject));
    }

    #[tokio::test]
    async fn test_get_function_no_exported_fn() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {}

export default { } // Empty object
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let function = loaded_code.into_function(false, "foo".to_string()).await;

        let err = function.err().unwrap();
        assert!(matches!(err, JSRunnerError::NoExportedFunction(_)));
    }

    #[tokio::test]
    async fn test_get_function_exported_if_not_a_function() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
const foo = 55

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let function = loaded_code.into_function(false, "foo".to_string()).await;

        let err = function.err().unwrap();
        assert!(matches!(err, JSRunnerError::ExportedElementNotAFunction));
    }

    #[tokio::test]
    async fn test_run_function_sync() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(1_000),
                },
            )
            .await
            .unwrap();
        assert_eq!(&ret, "foo");
    }

    #[tokio::test]
    async fn test_run_function_multiple_params() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, Vec<serde_json::Value>, String>(
            r#"
function foo(a, b, c) {
    return a + b + c
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();

        let params: Vec<serde_json::Value> = vec!["foo".into(), 32.into(), "gg".into()];
        let ret = function
            .exec(
                params,
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(1_000),
                },
            )
            .await
            .unwrap();
        assert_eq!(&ret, "foo32gg");
    }

    #[tokio::test]
    async fn test_run_function_async() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    return Promise.resolve(a)
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(1_000),
                },
            )
            .await
            .unwrap();
        assert_eq!(&ret, "foo");
    }

    #[tokio::test]
    async fn test_run_function_console_log_and_error() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {
    console.log('from log', a)
    console.error('from error', a)
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();

        let (sender, mut receiver) = tokio::sync::broadcast::channel(10);

        let ret = function
            .exec(
                "foo".to_string(),
                Some(Arc::new(sender)),
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(1_000),
                },
            )
            .await
            .unwrap();
        assert_eq!(&ret, "foo");

        let output = receiver.recv().await.unwrap();
        assert_eq!(
            output,
            (OutputChannel::StdOut, "from log foo\n".to_string())
        );

        let output = receiver.recv().await.unwrap();
        assert_eq!(
            output,
            (OutputChannel::StdErr, "from error foo\n".to_string())
        );
    }

    #[tokio::test]
    async fn test_run_function_sync_timeout() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    while (1) {}
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(500),
                },
            )
            .await;

        let err = ret.err().unwrap();
        assert!(matches!(err, JSRunnerError::ExecTimeout));

        assert!(!function.is_alive());
    }

    #[tokio::test]
    async fn test_run_function_async_timeout() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    await new Promise(r => setTimeout(r, 2000)) // 2 sec
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(500),
                },
            )
            .await;

        let err = ret.err().unwrap();
        assert!(matches!(err, JSRunnerError::ExecTimeout));
        assert!(!function.is_alive());
    }

    #[tokio::test]
    async fn test_run_function_really_async() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    await new Promise(r => setTimeout(r, 100)) // 100 ms
    return a
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await
            .unwrap();

        assert_eq!("foo".to_string(), ret);
    }

    #[tokio::test]
    async fn test_run_function_sync_thrown() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
function foo(a) {
    throw new Error('KABOOM')
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await;

        let err = ret.err().unwrap();
        let JSRunnerError::ErrorThrown(err) = err else {
            panic!("No JSRunnerError::ErrorThrown");
        };
        assert_eq!(err.message, Some("KABOOM".to_string()));
    }

    #[tokio::test]
    async fn test_run_function_async_thrown() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    throw new Error('KABOOM')
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await;

        let err = ret.err().unwrap();
        let JSRunnerError::ErrorThrown(err) = err else {
            panic!("No JSRunnerError::ErrorThrown");
        };
        assert_eq!(err.message, Some("KABOOM".to_string()));
    }

    #[tokio::test]
    async fn test_run_function_really_async_thrown() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    await new Promise(r => setTimeout(r, 10)) // 10 ms
    throw new Error('KABOOM')
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: None,
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await;

        let err = ret.err().unwrap();
        let JSRunnerError::ErrorThrown(err) = err else {
            panic!("No JSRunnerError::ErrorThrown");
        };
        assert_eq!(err.message, Some("KABOOM".to_string()));
    }

    #[tokio::test]
    async fn test_run_function_async_domain_not_allowed() {
        let _ = tracing_subscriber::fmt::try_init();

        let loaded_code = load_code::<String, String, String>(
            r#"
async function foo(a) {
    await fetch('http://localhost:3000/') // Not allowed
}

export default { foo }
            "#
            .to_string(),
            None,
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: Some(vec![]),
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await;

        let err = ret.err().unwrap();
        let JSRunnerError::ErrorThrown(err) = err else {
            panic!("No JSRunnerError::ErrorThrown");
        };
        assert!(err.message.unwrap().contains("Domain not allowed"));
    }

    #[tokio::test]
    async fn test_run_function_async_domain_allowed() {
        let _ = tracing_subscriber::fmt::try_init();

        let addr = start_http_server().await;

        let loaded_code = load_code::<String, String, String>(
            format!(
                r#"
async function foo(a) {{
    const res = await fetch('http://localhost:{}/') // Not allowed
    if (!res.ok) {{
        throw new Error('NOT OK')
    }}

    const output = await res.text()

    return `${{a}} ${{output}}`
}}

export default {{ foo }}
            "#,
                addr.port()
            ),
            Some(vec![]),
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(true, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                "foo".to_string(),
                None,
                ExecOption {
                    allowed_hosts: Some(vec![format!("localhost:{}", addr.port())]),
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await
            .unwrap();

        assert_eq!(ret, "foo Hello, World!".to_string());
    }

    #[tokio::test]
    async fn test_run_function_custom_struct() {
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
        struct Foo {
            name: String,
        }
        let loaded_code = load_code::<String, Foo, Option<Foo>>(
            r#"
function foo(a) {
    if (a.name === "pippo") {
        return null
    }
    if (a.name === "pluto") {
        return undefined
    }
    return a
}

export default { foo }
            "#
            .to_string(),
            Some(vec![]),
            Duration::from_millis(1_000),
            SharedCache::new(),
            SharedKV::new(),
            SharedSecrets::new(),
        )
        .await
        .unwrap();

        let mut function = loaded_code
            .into_function(false, "foo".to_string())
            .await
            .unwrap();

        let ret = function
            .exec(
                Foo {
                    name: "foo".to_string(),
                },
                None,
                ExecOption {
                    allowed_hosts: Some(vec![]),
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            ret,
            Some(Foo {
                name: "foo".to_string(),
            })
        );

        let ret = function
            .exec(
                Foo {
                    name: "pippo".to_string(),
                },
                None,
                ExecOption {
                    allowed_hosts: Some(vec![]),
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await
            .unwrap();

        assert_eq!(ret, None);

        let ret = function
            .exec(
                Foo {
                    name: "pluto".to_string(),
                },
                None,
                ExecOption {
                    allowed_hosts: Some(vec![]),
                    timeout: Duration::from_millis(5_000),
                },
            )
            .await
            .unwrap();

        assert_eq!(ret, None);
    }

    async fn start_http_server() -> SocketAddr {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::task::spawn(async {
            use axum::{routing::get, Router};
            use std::net::*;

            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            drop(listener);

            let app = Router::new().route("/", get(|| async { "Hello, World!" }));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

            let _ = sender.send(addr);

            axum::serve(listener, app).await.unwrap();
        });

        let addr = receiver.await.unwrap();

        addr
    }
}
