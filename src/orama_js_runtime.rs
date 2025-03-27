use core::panic;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::{
    pin, select,
    sync::{mpsc, oneshot, RwLock},
    task::JoinHandle,
};
use tracing::{error, info, trace};

use crate::pool::{Input, JSExecutorPool, JSExecutorPoolConfig, JSExecutorPoolError, Output};

pub struct OramaJSPoolConfig {
    pub pool_config: JSExecutorPoolConfig,
    pub max_idle_time: Duration,
    pub check_interval: Duration,
}

struct OramaJSPoolInner<Input, Output> {
    #[allow(clippy::type_complexity)]
    pools: HashMap<
        Vec<u8>,
        (
            // Pool
            JSExecutorPool<Input, Output>,
            // Access time
            // std::sync::Mutex and not tokio because the "blocking" operation is very fast
            Mutex<Instant>,
        ),
    >,
    closed: bool,
}

pub struct OramaJSPool<Input, Output> {
    pool_config: JSExecutorPoolConfig,
    inner: Arc<RwLock<OramaJSPoolInner<Input, Output>>>,
    count: Arc<AtomicUsize>,
    shutdown_tx: oneshot::Sender<()>,
    handler: JoinHandle<()>,
}

impl<I: Input, O: Output> OramaJSPool<I, O> {
    pub fn new(config: OramaJSPoolConfig) -> Self {
        let inner = Arc::new(RwLock::new(OramaJSPoolInner {
            pools: Default::default(),
            closed: false,
        }));

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let count = Arc::new(AtomicUsize::new(0));

        let inner_clone = inner.clone();
        let count_clone = count.clone();
        let handler = tokio::spawn(async move {
            pin! {
                let fut = shutdown_rx;
            };
            let mut interval = tokio::time::interval(config.check_interval);
            loop {
                select!(
                    _ = interval.tick() => {},
                    _ = &mut fut => {
                        info!("Shutting down OramaJSPool");
                        let mut lock = inner_clone.write().await;
                        lock.closed = true;
                        let keys = lock.pools.keys().cloned().collect::<Vec<_>>();
                        for key in keys {
                            if let Some(p) = lock.pools.remove(&key) {
                                p.0.close().await.unwrap();
                            }
                        }

                        count_clone.store(0, std::sync::atomic::Ordering::Relaxed);

                        break;
                    }
                );
                let removed_elements =
                    Self::remove_expired_pools(inner_clone.clone(), &config.max_idle_time).await;
                count_clone.fetch_sub(removed_elements, std::sync::atomic::Ordering::Relaxed);
            }
            info!("Shut down OramaJSPool");
        });

        Self {
            pool_config: config.pool_config,
            count,
            inner,
            shutdown_tx,
            handler,
        }
    }

    pub fn len(&self) -> usize {
        self.count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub async fn execute(&self, code: &str, input: I) -> Result<O, JSExecutorPoolError<I, O>> {
        let id = sha256_digest(code);

        trace!("Executing code with id: {:?}", id);
        let lock = self.inner.read().await;

        if lock.closed {
            return Err(JSExecutorPoolError::ShuttingDown);
        }

        let pool = lock.pools.get(&id);
        if let Some(p) = pool {
            *p.1.lock().unwrap() = Instant::now();
            return p.0.exec(input).await;
        };
        drop(lock);

        info!("Creating new pool for code with id: {:?}", id);

        let mut lock = self.inner.write().await;
        let pool = lock.pools.get(&id);
        // Concurrent write: avoid insert the pool twice
        if let Some(p) = pool {
            *p.1.lock().unwrap() = Instant::now();
            return p.0.exec(input).await;
        };
        let executor = JSExecutorPool::new(self.pool_config.clone(), code.to_string()).await?;
        lock.pools
            .insert(id.clone(), (executor, Mutex::new(Instant::now())));
        self.count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(lock);

        let lock = self.inner.read().await;
        let pool = lock.pools.get(&id);
        match pool {
            Some(p) => {
                *p.1.lock().unwrap() = Instant::now();
                return p.0.exec(input).await;
            }
            None => {
                panic!("Pool not found");
            }
        };
    }

    pub async fn execute_stream(
        &self,
        code: &str,
        input: I,
    ) -> Result<mpsc::UnboundedReceiver<O>, JSExecutorPoolError<I, O>> {
        let id = sha256_digest(code);

        trace!("Executing code with id: {:?}", id);
        let lock = self.inner.read().await;

        if lock.closed {
            return Err(JSExecutorPoolError::ShuttingDown);
        }

        let pool = lock.pools.get(&id);
        if let Some(p) = pool {
            *p.1.lock().unwrap() = Instant::now();
            return p.0.exec_stream(input).await;
        };
        drop(lock);

        info!("Creating new pool for code with id: {:?}", id);

        let mut lock = self.inner.write().await;
        let pool = lock.pools.get(&id);
        // Concurrent write: avoid insert the pool twice
        if let Some(p) = pool {
            *p.1.lock().unwrap() = Instant::now();
            return p.0.exec_stream(input).await;
        };
        let executor = JSExecutorPool::new(self.pool_config.clone(), code.to_string()).await?;
        lock.pools
            .insert(id.clone(), (executor, Mutex::new(Instant::now())));
        self.count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(lock);

        let lock = self.inner.read().await;
        let pool = lock.pools.get(&id);
        match pool {
            Some(p) => {
                *p.1.lock().unwrap() = Instant::now();
                return p.0.exec_stream(input).await;
            }
            None => {
                panic!("Pool not found");
            }
        };
    }

    pub async fn close(self) -> Result<(), JSExecutorPoolError<I, O>> {
        self.shutdown_tx.send(()).unwrap();
        self.handler.await.unwrap();

        let mut inner = self.inner.write().await;
        for (_, (pool, _)) in inner.pools.drain() {
            pool.close().await?;
        }

        Ok(())
    }

    async fn remove_expired_pools(
        inner: Arc<RwLock<OramaJSPoolInner<I, O>>>,
        max_idle_time: &Duration,
    ) -> usize {
        let inner = &mut *inner.write().await;
        let now = Instant::now();
        let mut to_remove = vec![];
        for (id, (_, access_time)) in inner.pools.iter() {
            let access_time = access_time.lock().unwrap();
            if now.duration_since(*access_time) > *max_idle_time {
                to_remove.push(id.clone());
            }
        }

        let removed_elements = to_remove.len();
        for id in to_remove {
            if let Some(r) = inner.pools.remove(&id) {
                match r.0.close().await {
                    Ok(_) => {
                        info!("Closed pool for code with id: {:?}", id);
                    }
                    Err(e) => {
                        error!("Error closing pool for code with id: {:?}: {:?}", id, e);
                    }
                }
            }
        }

        removed_elements
    }
}

fn sha256_digest(s: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let hash = Sha256::digest(s.as_bytes());
    hash.to_vec()
}

#[cfg(test)]
mod tests {
    use std::{sync::atomic::Ordering, time::Duration};

    use tokio::time::sleep;

    use crate::executor::JSExecutorConfig;

    use super::*;

    #[tokio::test]
    async fn test_pool_different_codes() {
        let _ = tracing_subscriber::fmt::try_init();

        let runtime = OramaJSPool::new(OramaJSPoolConfig {
            pool_config: JSExecutorPoolConfig {
                instances_count_per_code: 4,
                queue_capacity: 10,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_startup_time: Duration::from_millis(200),
                    max_execution_time: Duration::from_millis(300),
                    function_name: "m".to_string(),
                    is_async: false,
                },
            },
            max_idle_time: Duration::from_millis(300),
            check_interval: Duration::from_millis(60_000),
        });

        assert_eq!(runtime.len(), 0);

        let code1 = r#"
function m() {
    return 1 + 2;
}
export default { m };
"#;
        let code2 = r#"
function m() {
    return 3 + 4;
}
export default { m };
"#;

        let result: u8 = runtime.execute(code1, 1).await.unwrap();
        assert_eq!(result, 3);
        assert_eq!(runtime.len(), 1);

        let result: u8 = runtime.execute(code2, 1).await.unwrap();
        assert_eq!(result, 7);
        assert_eq!(runtime.len(), 2);

        let result: u8 = runtime.execute(code1, 1).await.unwrap();
        assert_eq!(result, 3);
        let result: u8 = runtime.execute(code1, 1).await.unwrap();
        assert_eq!(result, 3);
        let result: u8 = runtime.execute(code1, 1).await.unwrap();
        assert_eq!(result, 3);

        let result: u8 = runtime.execute(code2, 1).await.unwrap();
        assert_eq!(result, 7);
        let result: u8 = runtime.execute(code2, 1).await.unwrap();
        assert_eq!(result, 7);
        let result: u8 = runtime.execute(code2, 1).await.unwrap();
        assert_eq!(result, 7);
        let result: u8 = runtime.execute(code2, 1).await.unwrap();
        assert_eq!(result, 7);

        assert_eq!(runtime.len(), 2);

        // Pass time to remove expired pools
        sleep(Duration::from_millis(600)).await;

        let counter = runtime.count.clone();

        runtime.close().await.unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_pool_stream() {
        let _ = tracing_subscriber::fmt::try_init();

        let runtime: OramaJSPool<i32, u8> = OramaJSPool::new(OramaJSPoolConfig {
            pool_config: JSExecutorPoolConfig {
                instances_count_per_code: 1,
                queue_capacity: 1,
                executor_config: JSExecutorConfig {
                    allowed_hosts: vec![],
                    max_startup_time: Duration::from_millis(200),
                    max_execution_time: Duration::from_millis(1_000),
                    function_name: "m".to_string(),
                    is_async: true,
                },
            },
            max_idle_time: Duration::from_millis(300),
            check_interval: Duration::from_millis(60_000),
        });

        assert_eq!(runtime.len(), 0);

        let code1 = r#"
async function* m(a) {
    await new Promise((resolve) => setTimeout(resolve, 100));
    yield a;
    await new Promise((resolve) => setTimeout(resolve, 100));
    yield a + 1;
    await new Promise((resolve) => setTimeout(resolve, 100));
    yield a + 2;
}
export default { m };
"#;

        let mut result = runtime.execute_stream(code1, 1).await.unwrap();

        assert_eq!(runtime.len(), 1);

        assert_eq!(result.recv().await.unwrap(), 1);
        assert_eq!(result.recv().await.unwrap(), 2);
        assert_eq!(result.recv().await.unwrap(), 3);

        sleep(Duration::from_millis(100)).await;

        assert!(result.is_closed());

        let counter = runtime.count.clone();

        runtime.close().await.unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
