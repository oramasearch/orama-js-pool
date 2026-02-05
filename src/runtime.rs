use std::{marker::PhantomData, sync::Arc, thread::JoinHandle, time::Duration};

use deno_core::{
    error::CoreError, v8::IsolateHandle, ModuleCodeString, ModuleSpecifier, PollEventLoopOptions,
};
use deno_web::BlobStore;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::{runtime::Builder, task::LocalSet};
use tracing::{debug, warn};

use crate::{
    orama_extension::{ChannelStorage, OutputChannel, SharedCache, StdoutHandler, StdoutHandlerFn},
    permission::CustomPermissions,
    DomainPermission,
};

use super::parameters::TryIntoFunctionParameters;

deno_core::extension!(deno_telemetry, esm = ["telemetry.ts", "util.ts"]);

pub static RUNTIME_SNAPSHOT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

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
    #[error("The default export is not an object")]
    DefaultExportIsNotAnObject,
    #[error("Module '{0}' not found")]
    MissingModule(String),
    #[error("Exported function '{0}' not found in default export")]
    MissingExportedFunction(String),
    #[error("Export '{0}' exists but is not a function")]
    ExportIsNotAFunction(String),
    #[error("The script took too long to execute")]
    ExecTimeout,
    #[error("Network permission denied: {0}")]
    NetworkPermissionDenied(String),
    #[error("Parameter error: {0}")]
    ParameterError(#[from] serde_json::Error),
    #[error("Compilation error: {0}")]
    CompilationError(Box<deno_core::error::JsError>),
    #[error("The runtime has been terminated")]
    Terminated,
    #[error("Unknown error: {0}")]
    Unknown(String),
}

enum RuntimeEvent {
    Stop,
    LoadModule {
        specifier: String,
        code: String,
        sender: tokio::sync::oneshot::Sender<Result<(), RuntimeError>>,
    },
    ExecFunction {
        id: u64,
        module_specifier: String,
        function_name: String,
        input_params: String,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        domain_permission: DomainPermission,
        sender: tokio::sync::oneshot::Sender<Result<serde_json::Value, RuntimeError>>,
    },
}

use std::collections::HashMap;

/// Low-level runtime managing a single Deno JsRuntime instance with multiple modules
pub struct Runtime<Input, Output> {
    handler: IsolateHandle,
    join_handler: JoinHandle<()>,
    sender: tokio::sync::mpsc::Sender<RuntimeEvent>,
    exec_count: u64,
    should_recreate: bool,
    loaded_modules: HashMap<String, String>, // module_name -> specifier
    evaluation_timeout: Duration,
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
    pub async fn new(
        domain_permission: DomainPermission,
        evaluation_timeout: Duration,
        shared_cache: SharedCache,
    ) -> Result<Self, RuntimeError> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<RuntimeEvent>(1);
        let (init_sender1, init_receiver1) =
            tokio::sync::oneshot::channel::<Result<IsolateHandle, CoreError>>();
        let (init_sender2, init_receiver2) =
            tokio::sync::oneshot::channel::<Result<(), CoreError>>();

        let thread_id = std::thread::spawn(move || {
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime in Deno runtime");

            let local = LocalSet::new();
            local.spawn_local(async move {
                let blob_store = BlobStore::default();
                let blob_store = Arc::new(blob_store);

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
                            CustomPermissions {
                                domain_permission: domain_permission.clone(),
                            },
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

                init_sender2
                    .send(Ok(()))
                    .expect("Cannot send runtime ready signal");

                while let Some(ev) = receiver.recv().await {
                    debug!("Received event...");
                    match ev {
                        RuntimeEvent::Stop => {
                            warn!("Stopping loop due to received command");
                            break;
                        }
                        RuntimeEvent::LoadModule {
                            specifier,
                            code,
                            sender,
                        } => {
                            let result = load_module(&mut js_runtime, &specifier, code).await;
                            let _ = sender.send(result);
                        }
                        RuntimeEvent::ExecFunction {
                            id,
                            module_specifier,
                            function_name,
                            input_params,
                            stdout_sender,
                            domain_permission,
                            sender,
                        } => {
                            debug!("Overriding state");
                            update_inner_state(&mut js_runtime, stdout_sender, domain_permission);
                            debug!("State overridden");

                            let result = execute_function(
                                &mut js_runtime,
                                id,
                                &module_specifier,
                                &function_name,
                                &input_params,
                            )
                            .await;

                            // We do not close the runtime on error, it does not provide any advantage
                            let _ = sender.send(result);
                        }
                    };
                }
            });

            rt.block_on(local);
        });

        let handler = init_receiver1
            .await
            .expect("Failed to receive IsolateHandle from runtime initialization");
        let handler = match handler {
            Ok(handler) => handler,
            Err(e) => {
                warn!("{e:?}");
                return Err(RuntimeError::InitTimeout);
            }
        };

        let output = tokio::time::timeout(evaluation_timeout, init_receiver2).await;
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
            Ok(Ok(Ok(_))) => Ok(Self {
                handler,
                join_handler: thread_id,
                sender,
                exec_count: 0,
                should_recreate: false,
                loaded_modules: HashMap::new(),
                evaluation_timeout,
                _p: PhantomData,
            }),
        }
    }

    /// Load a module into the runtime
    pub async fn load_module<Code: Into<ModuleCodeString>>(
        &mut self,
        module_name: String,
        code: Code,
    ) -> Result<(), RuntimeError> {
        if !self.is_alive() {
            return Err(RuntimeError::Terminated);
        }

        let code: ModuleCodeString = code.into();
        let code_string = code.to_string();
        let specifier = format!("file:/{module_name}");

        let (sender, receiver) = tokio::sync::oneshot::channel();

        self.sender
            .send(RuntimeEvent::LoadModule {
                specifier: specifier.clone(),
                code: code_string,
                sender,
            })
            .await
            .expect("Failed to send LoadModule event to runtime");

        tokio::time::timeout(self.evaluation_timeout, receiver)
            .await
            .map_err(|_| {
                warn!("Module evaluation timeout for {module_name}");
                // Terminate to stop expensive module; worker will recreate runtime
                self.handler.terminate_execution();
                self.should_recreate = true;
                RuntimeError::InitTimeout
            })?
            .expect("Failed to receive LoadModule response from runtime")?;

        self.loaded_modules.insert(module_name, specifier);

        Ok(())
    }

    /// Execute a function with the given parameters in a specific module
    pub async fn exec(
        &mut self,
        module_name: &str,
        function_name: String,
        params: Input,
        stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
        domain_permission: DomainPermission,
        timeout: Duration,
    ) -> Result<Output, RuntimeError> {
        if !self.is_alive() {
            return Err(RuntimeError::Terminated);
        }

        let module_specifier = self
            .loaded_modules
            .get(module_name)
            .ok_or_else(|| RuntimeError::MissingModule(module_name.to_string()))?
            .clone();

        let id = self.exec_count;
        self.exec_count += 1;

        let params = params.try_into_function_parameter()?;
        let params = params.0;
        let input_params = serde_json::to_string(&params).map_err(RuntimeError::ParameterError)?;

        let (sender, receiver) =
            tokio::sync::oneshot::channel::<Result<serde_json::Value, RuntimeError>>();

        self.sender
            .send(RuntimeEvent::ExecFunction {
                id,
                module_specifier,
                function_name,
                input_params,
                stdout_sender,
                domain_permission,
                sender,
            })
            .await
            .expect("Failed to send ExecFunction event to runtime");

        let output = tokio::time::timeout(timeout, receiver).await;

        let output = match output {
            Err(_) => {
                self.should_recreate = true;
                self.handler.terminate_execution();
                return Err(RuntimeError::ExecTimeout);
            }
            Ok(Err(_)) => {
                unreachable!("Receiver error");
            }
            Ok(Ok(Ok(t))) => t,
            Ok(Ok(Err(e))) => {
                return Err(e);
            }
        };

        let output: Output =
            serde_json::from_value(output).map_err(RuntimeError::ParameterError)?;

        Ok(output)
    }

    /// Check if the runtime is still alive
    pub fn is_alive(&self) -> bool {
        !self.join_handler.is_finished() && !self.should_recreate
    }
}

async fn load_module(
    js_runtime: &mut deno_core::JsRuntime,
    specifier: &str,
    code: String,
) -> Result<(), RuntimeError> {
    let specifier =
        ModuleSpecifier::parse(specifier).expect("Module specifier should be valid URL format");

    let mod_id = js_runtime
        .load_side_es_module_from_code(&specifier, code)
        .await
        .map_err(|e| match e {
            CoreError::Js(js_err) => {
                if js_err.name.as_ref().is_some_and(|s| s == "SyntaxError") {
                    RuntimeError::CompilationError(Box::new(js_err))
                } else {
                    RuntimeError::InitializationError(Box::new(CoreError::Js(js_err)))
                }
            }
            _ => RuntimeError::InitializationError(Box::new(e)),
        })?;

    let eval = js_runtime.mod_evaluate(mod_id);

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| RuntimeError::InitializationError(Box::new(e)))?;

    eval.await
        .map_err(|e| RuntimeError::InitializationError(Box::new(e)))?;

    Ok(())
}

async fn execute_function(
    js_runtime: &mut deno_core::JsRuntime,
    id: u64,
    module_specifier: &str,
    function_name: &str,
    input_params: &str,
) -> Result<serde_json::Value, RuntimeError> {
    // Unique specifier prevents Deno's module cache from reusing previous execution results
    let exec_specifier = ModuleSpecifier::parse(&format!("file:/exec_{id}"))
        .expect("Execution specifier should be valid URL format");

    // Integrated function checking and execution in a single JS evaluation.
    // This checks if:
    // 1. The default export is an object
    // 2. The function exists in the default export
    // 3. The function is actually callable
    // If checks pass, we execute the function. Otherwise, we set an error code.
    //
    // Conditionally await the function result to avoid overhead for sync functions.
    // We check if the result is async using two conditions:
    // 1. instanceof Promise - catches native Promises from async functions
    // 2. thenable check (has a .then method) - catches custom Promise-like objects
    // For synchronous functions, we avoid the microtask scheduling overhead of await.
    let code = format!(
        r#"
import main from "{module_specifier}";

if (typeof main !== 'object') {{
    globalThis.{GLOBAL_VARIABLE_NAME} = {{ __error: 1 }};
}} else if (!main.{function_name}) {{
    globalThis.{GLOBAL_VARIABLE_NAME} = {{ __error: 2 }};
}} else if (typeof main.{function_name} !== 'function') {{
    globalThis.{GLOBAL_VARIABLE_NAME} = {{ __error: 3 }};
}} else {{
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
}}
        "#,
    );

    let mod_id = match js_runtime
        .load_side_es_module_from_code(&exec_specifier, code)
        .await
    {
        Ok(mod_id) => mod_id,
        Err(e) => {
            return match e {
                CoreError::Js(js_err) => {
                    if js_err.name.as_ref().is_some_and(|s| s == "SyntaxError") {
                        Err(RuntimeError::CompilationError(Box::new(js_err)))
                    } else {
                        Err(RuntimeError::ErrorThrown(Box::new(js_err)))
                    }
                }
                _ => Err(RuntimeError::UnknownExecutionError(Box::new(e))),
            };
        }
    };
    debug!("Evaluating code");

    let eval = js_runtime.mod_evaluate(mod_id);

    match js_runtime.run_event_loop(Default::default()).await {
        Ok(_) => {}
        Err(e) => {
            return match e {
                CoreError::Js(e) => {
                    // Check if this is a network permission error
                    if let Some(msg) = &e.message {
                        if msg
                            .contains(crate::permission::DOMAIN_NOT_ALLOWED_ERROR_MESSAGE_SUBSTRING)
                        {
                            return Err(RuntimeError::NetworkPermissionDenied(msg.clone()));
                        }
                    }
                    Err(RuntimeError::ErrorThrown(Box::new(e)))
                }
                _ => Err(RuntimeError::UnknownExecutionError(Box::new(e))),
            };
        }
    };

    match eval.await {
        Ok(_) => {}
        Err(e) => {
            return Err(RuntimeError::UnknownExecutionError(Box::new(e)));
        }
    };

    let mut scope: deno_core::v8::HandleScope<'_> = js_runtime.handle_scope();
    let context = scope.get_current_context();
    let global = context.global(&mut scope);
    let key = deno_core::v8::String::new(&mut scope, GLOBAL_VARIABLE_NAME)
        .expect("Failed to create V8 string for global variable name");
    let value = global
        .get(&mut scope, key.into())
        .expect("Failed to get global variable from V8 context");
    let output: serde_json::Value = deno_core::serde_v8::from_v8(&mut scope, value)
        .expect("Failed to deserialize V8 value to JSON");

    // Check if the output contains a function validation error
    if let Some(obj) = output.as_object() {
        if let Some(error_code) = obj.get("__error").and_then(|v| v.as_u64()) {
            return match error_code {
                1 => Err(RuntimeError::DefaultExportIsNotAnObject),
                2 => Err(RuntimeError::MissingExportedFunction(
                    function_name.to_string(),
                )),
                3 => Err(RuntimeError::ExportIsNotAFunction(
                    function_name.to_string(),
                )),
                _ => unreachable!(),
            };
        }
    }

    Ok(output)
}

fn update_inner_state(
    js_runtime: &mut deno_core::JsRuntime,
    stdout_sender: Option<Arc<tokio::sync::broadcast::Sender<(OutputChannel, String)>>>,
    domain_permission: DomainPermission,
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
    state.put(CustomPermissions { domain_permission });
    drop(rc_state_ref);
    drop(rc_state);
}
