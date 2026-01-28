use std::{marker::PhantomData, sync::Arc, thread::JoinHandle, time::Duration};

use deno_core::{
    error::CoreError, v8::IsolateHandle, ModuleCodeString, ModuleSpecifier, PollEventLoopOptions,
};
use deno_web::BlobStore;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::{runtime::Builder, task::LocalSet};
use tracing::{debug, info, warn};

use crate::{
    orama_extension::{ChannelStorage, OutputChannel, SharedCache, StdoutHandler, StdoutHandlerFn},
    parameters::TryIntoFunctionParameters,
    permission::CustomPermissions,
    runner::JSRunnerError,
};

deno_core::extension!(deno_telemetry, esm = ["telemetry.ts", "util.ts"],);

pub static RUNTIME_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

const MAIN_IMPORT_MODULE_NAME: &str = "file:/main";
const GLOBAL_VARIABLE_NAME: &str = "__result";

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Cannot start runtime: {0}")]
    InitializationError(Box<deno_core::error::CoreError>),
    #[error("A JS error is thrown: {0}")]
    ErrorThrown(Box<deno_core::error::JsError>),
    #[error("Unknown execution error: {0}")]
    UnknownExecutionError(Box<deno_core::error::CoreError>),
    #[error("The JS initialization took too long")]
    InitTimeout,
    #[error("The script has a default export but it is not an object")]
    DefaultExportIsNotAnObject,
    #[error("The script has a default export but it does not provide an export named '{0}'")]
    NoExportedFunction(String),
    #[error("The script has a default export with the correct name, but type is wrong. Expected a function")]
    ExportedElementNotAFunction,
    #[error("The script took too long to execute")]
    ExecTimeout,
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("compilation error: {0}")]
    CompilationError(Box<deno_core::error::JsError>),
    #[error("Parameter error: {0}")]
    ParameterError(#[from] JSRunnerError),
    #[error("unknown")]
    Unknown,
}

enum RuntimeEvent {
    Stop,
    CheckFunction(String, tokio::sync::oneshot::Sender<u8>),
    ExecFunction {
        id: u64,
        function_name: String,
        input_params: String,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        allowed_hosts: Option<Vec<String>>,
        sender: tokio::sync::oneshot::Sender<Result<serde_json::Value, RuntimeError>>,
    },
}

/// Low-level runtime managing a single Deno JsRuntime instance
pub struct Runtime<Input, Output> {
    handler: IsolateHandle,
    join_handler: JoinHandle<()>,
    sender: tokio::sync::mpsc::Sender<RuntimeEvent>,
    exec_count: u64,
    timed_out: bool,
    _p: PhantomData<(Input, Output)>,
}

impl<Input, Output> Drop for Runtime<Input, Output> {
    fn drop(&mut self) {
        self.handler.terminate_execution();
    }
}

impl<Input: TryIntoFunctionParameters + Send, Output: DeserializeOwned + Send + 'static>
    Runtime<Input, Output>
{
    /// Create a new runtime with the given JS code
    pub async fn new<Code: Into<ModuleCodeString> + Send + 'static>(
        code: Code,
        allowed_hosts: Option<Vec<String>>,
        timeout: Duration,
        shared_cache: SharedCache,
    ) -> Result<Self, RuntimeError> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<RuntimeEvent>(1);
        let (init_sender1, init_receiver1) =
            tokio::sync::oneshot::channel::<Result<IsolateHandle, CoreError>>();
        let (init_sender2, init_receiver2) =
            tokio::sync::oneshot::channel::<Result<(), CoreError>>();

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
                        deno_webidl::deno_webidl::init_ops(),
                        deno_url::deno_url::init_ops(),
                        deno_console::deno_console::init_ops(),
                        deno_web::deno_web::init_ops::<CustomPermissions>(blob_store, None),
                        deno_net::deno_net::init_ops::<CustomPermissions>(None, None),
                        deno_fetch::deno_fetch::init_ops::<CustomPermissions>(
                            deno_fetch::Options::default(),
                        ),
                        deno_crypto::deno_crypto::init_ops(None),
                        crate::orama_extension::orama_extension::init_ops(
                            CustomPermissions { allowed_hosts },
                            ChannelStorage::<Output> {
                                stream_handler: None,
                            },
                            StdoutHandler(None),
                            shared_cache,
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

                let m = ModuleSpecifier::parse(MAIN_IMPORT_MODULE_NAME).unwrap();
                let mod_id = js_runtime.load_main_es_module_from_code(&m, code).await;

                let mod_id = match mod_id {
                    Ok(mod_id) => mod_id,
                    Err(e) => {
                        warn!("Cannot load main ES module");
                        init_sender2.send(Err(e)).expect("Cannot send error");
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
                    .expect("Cannot send mod_evaluate ok init 2");

                info!("Waiting for runtime events...");
                while let Some(ev) = receiver.recv().await {
                    debug!("Received event...");
                    match ev {
                        RuntimeEvent::Stop => {
                            warn!("Stopping loop due to received command");
                            break;
                        }
                        RuntimeEvent::CheckFunction(function_name, sender) => {
                            let result = check_function(&mut js_runtime, &function_name).await;
                            let _ = sender.send(result);
                        }
                        RuntimeEvent::ExecFunction {
                            id,
                            function_name,
                            input_params,
                            stdout_sender,
                            allowed_hosts,
                            sender,
                        } => {
                            debug!("Overriding state");
                            update_inner_state(&mut js_runtime, stdout_sender, allowed_hosts);
                            debug!("State overridden");

                            let result = execute_function(
                                &mut js_runtime,
                                id,
                                &function_name,
                                &input_params,
                            )
                            .await;

                            let is_err = result.is_err();
                            let _ = sender.send(result);

                            if is_err {
                                // Stop runtime on execution error
                                return;
                            }
                        }
                    };
                }

                info!("Runtime task stopped")
            });

            rt.block_on(local);

            info!("Runtime thread stopped")
        });

        let handler = init_receiver1.await.unwrap();
        let handler = match handler {
            Ok(handler) => handler,
            Err(e) => {
                warn!("{e:?}");
                return Err(RuntimeError::InitTimeout);
            }
        };

        let output = tokio::time::timeout(timeout, init_receiver2).await;
        match output {
            Err(_) => {
                warn!("Startup took too much time. Terminating.");
                handler.terminate_execution();
                let _ = sender.send(RuntimeEvent::Stop).await;
                let _ = thread_id.join();
                Err(RuntimeError::InitTimeout)
            }
            Ok(Err(e)) => {
                panic!("RecvError {e:?}")
            }
            Ok(Ok(Err(e))) => {
                warn!("Error in startup");
                handler.terminate_execution();
                let _ = sender.send(RuntimeEvent::Stop).await;
                let _ = thread_id.join();

                match e {
                    CoreError::Js(e) => {
                        if e.name.as_ref().is_some_and(|s| s == "SyntaxError") {
                            return Err(RuntimeError::CompilationError(Box::new(e)));
                        }
                        Err(RuntimeError::InitializationError(Box::new(CoreError::Js(
                            e,
                        ))))
                    }
                    _ => Err(RuntimeError::InitializationError(Box::new(e))),
                }
            }
            Ok(Ok(Ok(_))) => {
                info!("Runtime loaded successfully");
                Ok(Self {
                    handler,
                    join_handler: thread_id,
                    sender,
                    exec_count: 0,
                    timed_out: false,
                    _p: PhantomData,
                })
            }
        }
    }

    /// Check if a function exists and is callable
    pub async fn check_function(
        &mut self,
        function_name: String,
        _is_async: bool,
    ) -> Result<(), RuntimeError> {
        if !self.is_alive() {
            return Err(RuntimeError::InitTimeout);
        }

        self.exec_count += 1;

        let (sender, receiver) = tokio::sync::oneshot::channel::<u8>();

        self.sender
            .send(RuntimeEvent::CheckFunction(function_name.clone(), sender))
            .await
            .unwrap();

        let output = receiver.await.unwrap();

        match output {
            0 => Ok(()),
            1 => Err(RuntimeError::DefaultExportIsNotAnObject),
            2 => Err(RuntimeError::NoExportedFunction(function_name)),
            3 => Err(RuntimeError::ExportedElementNotAFunction),
            _ => unreachable!(),
        }
    }

    /// Execute a function with the given parameters
    pub async fn exec(
        &mut self,
        function_name: String,
        params: Input,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        allowed_hosts: Option<Vec<String>>,
        timeout: Duration,
    ) -> Result<Output, RuntimeError> {
        if !self.is_alive() {
            return Err(RuntimeError::InitTimeout);
        }

        let id = self.exec_count;
        self.exec_count += 1;

        let params = params.try_into_function_parameter()?;
        let params = params.0;
        let input_params = serde_json::to_string(&params).map_err(RuntimeError::SerdeError)?;

        let (sender, receiver) =
            tokio::sync::oneshot::channel::<Result<serde_json::Value, RuntimeError>>();

        info!("Ask to exec");
        self.sender
            .send(RuntimeEvent::ExecFunction {
                id,
                function_name,
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
                return Err(RuntimeError::ExecTimeout);
            }
            Ok(Err(_)) => {
                unreachable!("Receiver error");
            }
            Ok(Ok(Ok(t))) => t,
            Ok(Ok(Err(e))) => return Err(e),
        };

        info!("Executed");

        let output: Output = serde_json::from_value(output).map_err(RuntimeError::SerdeError)?;

        Ok(output)
    }

    /// Check if the runtime is still alive
    pub fn is_alive(&self) -> bool {
        !(self.timed_out || self.join_handler.is_finished())
    }
}

async fn check_function(js_runtime: &mut deno_core::JsRuntime, function_name: &str) -> u8 {
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
            if e.name.as_ref() == Some(&"SyntaxError".to_string())
                && e.message
                    .as_ref()
                    .is_some_and(|s| s.contains("does not provide an export named 'default'"))
            {
                return 1;
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
    let key = deno_core::v8::String::new(&mut scope, GLOBAL_VARIABLE_NAME).unwrap();
    let value = global.get(&mut scope, key.into()).unwrap();
    deno_core::serde_v8::from_v8(&mut scope, value).unwrap()
}

async fn execute_function(
    js_runtime: &mut deno_core::JsRuntime,
    id: u64,
    function_name: &str,
    input_params: &str,
) -> Result<serde_json::Value, RuntimeError> {
    let specifier = ModuleSpecifier::parse(&format!("file:/{id}")).unwrap();

    // Conditionally await the function result to avoid overhead for sync functions.
    // We check if the result is async using two conditions:
    // 1. instanceof Promise - catches native Promises from async functions
    // 2. thenable check (has a .then method) - catches custom Promise-like objects
    // This comprehensive check handles:
    // - Native Promises from async functions
    // - Thenable objects (custom Promise-like objects with .then())
    // - Promises from transpiled/bundled code that may use different Promise implementations
    // For synchronous functions, we avoid the microtask scheduling overhead of await.
    let code = format!(
        r#"
import main from "{MAIN_IMPORT_MODULE_NAME}";
const thisContext = {{
    context: {{
        cache: {{
            get: (key) => Deno.core.ops.op_cache_get(key) ?? undefined,
            set: (key, value, options) => Deno.core.ops.op_cache_set(key, value, options?.ttl),
            delete: (key) => Deno.core.ops.op_cache_delete(key)
        }},
    }}
}};

const result = main.{function_name}.call(thisContext, ...{input_params});
const isAsync = result instanceof Promise || (result && typeof result.then === 'function');
globalThis.{GLOBAL_VARIABLE_NAME} = isAsync ? await result : result;
        "#,
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
            return match e {
                CoreError::Js(e) => Err(RuntimeError::ErrorThrown(Box::new(e))),
                _ => Err(RuntimeError::UnknownExecutionError(Box::new(e))),
            };
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
    let key = deno_core::v8::String::new(&mut scope, GLOBAL_VARIABLE_NAME).unwrap();
    let value = global.get(&mut scope, key.into()).unwrap();
    let output: serde_json::Value = deno_core::serde_v8::from_v8(&mut scope, value).unwrap();

    Ok(output)
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
