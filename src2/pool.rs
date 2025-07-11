use std::{
    any::Any,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use thiserror::Error;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::LocalSet,
};
use tracing::{debug, info, warn};

use crate::executor::{IntoFunctionParameters, JSExecutor, JSExecutorConfig, JSExecutorError};

pub trait Input: Send + Sync + IntoFunctionParameters + Debug + 'static {}
pub trait Output: Send + Sync + serde::de::DeserializeOwned + Debug + 'static {}

impl<T> Input for T where T: Send + Sync + IntoFunctionParameters + Debug + 'static {}
impl<T> Output for T where T: Send + Sync + serde::de::DeserializeOwned + Debug + 'static {}

#[derive(Error, Debug)]
pub enum JSExecutorPoolError<Input, Output> {
    #[error("Cannot start JSExecutorPool: {0}")]
    RuntimeInitializationError(std::io::Error),
    #[error("Cannot send item to executor: {0}")]
    ChannelError(#[from] async_channel::SendError<Action<Input, Output>>),
    #[error("Cannot receive reply from executor: {0}")]
    ReplyError(#[from] oneshot::error::RecvError),
    #[error("Execution error: {0}")]
    ExecutionError(#[from] JSExecutorError),
    #[error("Cannot close JSExecutorPool")]
    CannotClose,
    #[error("JSExecutorPool is shutting down")]
    ShuttingDown,
    #[error("Error joining thread")]
    CannotJoinThread(Box<dyn Any + Send + 'static>),
}

pub enum Action<Input, Output> {
    Exec(Item<Input, Output>),
    ExecStream(ItemStream<Input, Output>),
    Close,
}

pub struct Item<Input, Output> {
    input: Input,
    oneshot: oneshot::Sender<Result<Output, JSExecutorPoolError<Input, Output>>>,
}

pub struct ItemStream<Input, Output> {
    input: Input,
    sender: mpsc::UnboundedSender<Output>,
}

#[derive(Clone)]
pub struct JSExecutorPoolConfig {
    pub instances_count_per_code: usize,
    pub queue_capacity: usize,
    pub executor_config: JSExecutorConfig,
}

pub struct JSExecutorPool<Input, Output> {
    sender: async_channel::Sender<Action<Input, Output>>,
    is_closed: Arc<AtomicBool>,
    handlers: Vec<std::thread::JoinHandle<()>>,
}

impl<I: Input, O: Output> JSExecutorPool<I, O> {
    pub async fn new(
        config: JSExecutorPoolConfig,
        code: String,
    ) -> Result<Self, JSExecutorPoolError<I, O>> {
        let instances_count = config.instances_count_per_code;
        let executor_config = config.executor_config;

        let (sender, r) = async_channel::bounded(config.queue_capacity);

        let is_closed = Arc::new(AtomicBool::new(false));

        let mut handlers = vec![];
        for _ in 0..instances_count {
            let r = r.clone();
            let executor_config = executor_config.clone();
            let is_closed = is_closed.clone();

            let code = code.clone();

            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(JSExecutorPoolError::RuntimeInitializationError)?;
            let handler = std::thread::spawn(move || loop {
                let r1 = r.clone();
                start_loop(r1, &executor_config, &rt, code.clone());

                if is_closed.load(Ordering::Relaxed) {
                    break;
                }
                info!("JSExecutorPool loop stopped. Restarting...");
            });
            handlers.push(handler);
        }

        Ok(Self {
            sender,
            is_closed,
            handlers,
        })
    }

    pub async fn exec(&self, input: I) -> Result<O, JSExecutorPoolError<I, O>> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(JSExecutorPoolError::ShuttingDown);
        }

        let (tx, rx) = oneshot::channel();
        let send_result = self
            .sender
            .send(Action::Exec(Item { input, oneshot: tx }))
            .await;
        if let Err(err) = send_result {
            return Err(JSExecutorPoolError::ChannelError(err));
        }

        rx.await.map_err(JSExecutorPoolError::ReplyError)?
    }

    pub async fn exec_stream(
        &self,
        input: I,
    ) -> Result<mpsc::UnboundedReceiver<O>, JSExecutorPoolError<I, O>> {
        if self.is_closed.load(Ordering::Relaxed) {
            return Err(JSExecutorPoolError::ShuttingDown);
        }

        let (sender, receiver) = mpsc::unbounded_channel();
        let send_result = self
            .sender
            .send(Action::ExecStream(ItemStream { input, sender }))
            .await;
        if let Err(err) = send_result {
            return Err(JSExecutorPoolError::ChannelError(err));
        }

        Ok(receiver)
    }

    pub async fn close(self) -> Result<(), JSExecutorPoolError<I, O>> {
        self.is_closed.store(true, Ordering::Relaxed);
        self.sender
            .send(Action::Close)
            .await
            .map_err(JSExecutorPoolError::ChannelError)?;
        self.sender.close();

        let mut errors: Vec<Option<Box<dyn Any + Send>>> = vec![];
        for handler in self.handlers {
            match handler.join() {
                Ok(_) => errors.push(None),
                Err(err) => errors.push(Some(err)),
            };
        }

        let some_error = errors.iter().any(|e| e.is_some());
        if some_error {
            return Err(JSExecutorPoolError::CannotJoinThread(Box::new(errors)));
        }

        Ok(())
    }
}

fn start_loop<I: Input, O: Output>(
    r: async_channel::Receiver<Action<I, O>>,
    executor_config: &JSExecutorConfig,
    rt: &tokio::runtime::Runtime,
    code: String,
) {
    let local = LocalSet::new();

    let executor_config = executor_config.clone();
    local.spawn_local(async move {
        let mut executor = JSExecutor::try_new(executor_config, code).await.unwrap();

        debug!("Starting receiver items");
        while let Ok(action) = r.recv_blocking() {
            match action {
                Action::Close => {
                    info!("Received close action");
                    break;
                }
                Action::ExecStream(item_stream) => {
                    let ItemStream { input, sender } = item_stream;
                    let err = inner_exec_stream(&mut executor, input, move |v| {
                        if let Err(err) = sender.send(v) {
                            warn!("Error sending reply: {:?}", err);
                        }
                    })
                    .await;

                    match err {
                        Ok(_) => {}
                        Err(err) => {
                            warn!("Stopping the loop due to JS error: {:?}", err);
                            break;
                        }
                    };
                }
                Action::Exec(item) => {
                    match inner_exec(&mut executor, item.input).await {
                        Ok(output) => {
                            match item.oneshot.send(Ok(output)) {
                                Ok(_) => continue,
                                Err(err) => {
                                    warn!("Error sending reply: {:?}", err);
                                    info!("Stopping the loop due to channel error");
                                    break;
                                }
                            };
                        }
                        Err(err) => {
                            warn!("Stopping the loop due to JS error: {:?}", err);
                            match item.oneshot.send(Err(err)) {
                                Ok(_) => {
                                    break;
                                }
                                Err(err) => {
                                    warn!("Error sending reply: {:?}", err);
                                    info!("Stopping the loop due to channel error");
                                    break;
                                }
                            };
                        }
                    };
                }
            };
        }
    });

    rt.block_on(local);
}

async fn inner_exec<I: Input, O: Output>(
    executor: &mut JSExecutor<I, O>,
    input: I,
) -> Result<O, JSExecutorPoolError<I, O>> {
    executor
        .exec(input)
        .await
        .map_err(JSExecutorPoolError::ExecutionError)
}

async fn inner_exec_stream<I: Input, O: Output, F>(
    executor: &mut JSExecutor<I, O>,
    input: I,
    handler: F,
) -> Result<(), JSExecutorPoolError<I, O>>
where
    F: FnMut(O) + 'static,
{
    executor
        .exec_stream(input, handler)
        .await
        .map_err(JSExecutorPoolError::ExecutionError)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tokio::time::sleep;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool() {
        let pool = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 10,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(1),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: false,
                },
            },
            r#"
function myFunction(a) {
    return a;
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        for i in 0..100 {
            let output: i32 = pool.exec(i).await.unwrap();

            assert_eq!(output, i);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool_stream() {
        let pool: JSExecutorPool<i32, String> = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 1,
                queue_capacity: 1,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(1),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: true,
                },
            },
            r#"
async function* myFunction(a) {
    await new Promise(resolve => setTimeout(resolve, 100));
    yield `${a++}`;
    await new Promise(resolve => setTimeout(resolve, 100));
    yield `${a++}`;
    await new Promise(resolve => setTimeout(resolve, 100));
    yield `${a++}`;
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        let mut rec = pool.exec_stream(1).await.unwrap();

        assert_eq!(rec.recv().await.unwrap(), "1".to_string());
        assert_eq!(rec.recv().await.unwrap(), "2".to_string());
        assert_eq!(rec.recv().await.unwrap(), "3".to_string());

        sleep(Duration::from_millis(300)).await;

        assert!(rec.is_closed());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool_parallel_execution() {
        let pool: JSExecutorPool<i32, i32> = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 2,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(2),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: true,
                },
            },
            r#"
async function myFunction(a) {
    await new Promise(resolve => setTimeout(resolve, 400));
    return a;
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        let start: Instant = Instant::now();
        let (output1, output2) = tokio::join!(pool.exec(1), pool.exec(2),);
        let elapsed: Duration = start.elapsed();
        assert!(matches!(output1, Ok(1)));
        assert!(matches!(output2, Ok(2)));

        assert!(elapsed.as_millis() < 400 * 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool_queue_when_no_space_available_in_queue() {
        let pool: JSExecutorPool<i32, i32> = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 1,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(2),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: true,
                },
            },
            r#"
async function myFunction(a) {
    await new Promise(resolve => setTimeout(resolve, 400));
    return a;
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        let start: Instant = Instant::now();
        let (output1, output2) = tokio::join!(pool.exec(1), pool.exec(2),);
        let elapsed: Duration = start.elapsed();
        assert!(matches!(output1, Ok(1)));
        assert!(matches!(output2, Ok(2)));

        assert!(elapsed.as_millis() > 400 * 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool_error() {
        let pool: JSExecutorPool<i32, i32> = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 1,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(2),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: true,
                },
            },
            r#"
async function myFunction(a) {
    throw new Error()
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        // This crashes the unique instance in the pool.
        // When it happen, the pool should create a new instance.\
        for _ in 0..100 {
            let a = pool.exec(1).await;
            assert!(matches!(a, Err(JSExecutorPoolError::ExecutionError(_))));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_pool_close() {
        let pool: JSExecutorPool<i32, i32> = JSExecutorPool::new(
            JSExecutorPoolConfig {
                instances_count_per_code: 5,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_execution_time: Duration::from_secs(2),
                    max_startup_time: Duration::from_millis(300),
                    function_name: "myFunction".to_string(),
                    is_async: true,
                },
            },
            r#"
async function myFunction(a) {
    return a;
}
export default { myFunction };
"#
            .to_string(),
        )
        .await
        .unwrap();

        let a = pool.exec(1).await;
        assert!(matches!(a, Ok(1)));

        let a = pool.exec(1).await;
        assert!(matches!(a, Ok(1)));

        pool.close().await.unwrap();
    }
}
