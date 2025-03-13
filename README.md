# Orama JS Pool

[![Rust](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml/badge.svg)](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml)

This crate allows you to create a pool of JavaScript engines using the [Deno](https://deno.land/) runtime.
Internally, it uses the [deno_core](https://crates.io/crates/deno_core) crate to create the engines. Only some `extensions` are enabled by default.

## Usage

The pool is created one per code (it is used a sha256 hash of the code as identifier). This means that if you want to execute the same code with different arguments, the pool will be reused.

```rust
use std::time::Duration;

use orama_js_pool::{JSExecutorConfig, JSExecutorPoolConfig, OramaJSPool, OramaJSPoolConfig};

static CODE_SUM: &str = r#"
function sum(a, b) {
    return a + b
}
export default { sum };
"#;

static CODE_SUM_WITH_BUG: &str = r#"
function sum(a, b) {
    return 1 + a + b
}
export default { sum };
"#;

#[tokio::main]
async fn main() {
    let pool = OramaJSPool::new(OramaJSPoolConfig {
        pool_config: JSExecutorPoolConfig {
            instances_count_per_code: 2,
            queue_capacity: 10,
            executor_config: JSExecutorConfig {
                allowed_hosts: vec![],
                max_startup_time: std::time::Duration::from_millis(200),
                max_execution_time: std::time::Duration::from_millis(200),
                function_name: "sum".to_string(),
                is_async: false,
            },
        },
        // Close the pool if no activity for 60 seconds
        max_idle_time: Duration::from_secs(60),
        // Check every second if there are idle pools to close
        check_interval: Duration::from_secs(1),
    });

    // Run CODE_SUM code and execute `sum` function with 1 and 2 as arguments.
    // This starts a dedicated pool of workers (`instances_count_per_code`)
    // to process requests in parallel and execute the code
    let output: u8 = pool.execute(CODE_SUM, vec![1, 2]).await.unwrap();
    println!("sum(1, 2) == {}", output);
    assert_eq!(output, 3);

    // Run CODE_SUM code and execute `sum` function with 3 and 4 as arguments.
    // This time the computation will be more efficient because:
    // - the pool is already started
    // - the code is already loaded
    let output: u8 = pool.execute(CODE_SUM, vec![3, 4]).await.unwrap();
    println!("sum(3, 4) == {}", output);
    assert_eq!(output, 7);

    // Run CODE_SUM_2 starting a new dedicated pool of workers
    let output: u8 = pool.execute(CODE_SUM_WITH_BUG, vec![1, 2]).await.unwrap();
    println!("bugged code: sum(1, 2) == {}", output);
    assert_eq!(output, 4);

    // Close all pools
    pool.close().await.unwrap();
}
```

Orama JS Pool can run also async functions:

```rust
use std::time::Duration;

use orama_js_pool::{JSExecutorConfig, JSExecutorPoolConfig, OramaJSPool, OramaJSPoolConfig};

static CODE_ASYNC_SUM: &str = r#"
async function async_sum(a, b) {
    await new Promise(resolve => setTimeout(resolve, 100));
    return a + b
}
export default { async_sum };
"#;

#[tokio::main]
async fn main() {
    let pool = OramaJSPool::new(OramaJSPoolConfig {
        pool_config: JSExecutorPoolConfig {
            instances_count_per_code: 2,
            queue_capacity: 10,
            executor_config: JSExecutorConfig {
                allowed_hosts: vec![],
                max_startup_time: std::time::Duration::from_millis(200),
                max_execution_time: std::time::Duration::from_millis(200),
                function_name: "async_sum".to_string(),
                is_async: true,
            },
        },
        max_idle_time: Duration::from_secs(60),
        check_interval: Duration::from_secs(1),
    });

    let output: u8 = pool.execute(CODE_ASYNC_SUM, vec![1, 2]).await.unwrap();
    println!("async_sum(1, 2) == {}", output);
    assert_eq!(output, 3);

    // Close all pools
    pool.close().await.unwrap();
}
```

Orama JS Pool handles also async iterators:

```rust
use std::time::Duration;

use orama_js_pool::{JSExecutorConfig, JSExecutorPoolConfig, OramaJSPool, OramaJSPoolConfig};

static CODE_COUNT: &str = r#"
async function* count(m, s) {
    for (let i = 1; i <= m; i++) {
        await new Promise((resolve) => setTimeout(resolve, s));
        yield i;
    }
}
export default { count };
"#;

#[tokio::main]
async fn main() {
    let pool: OramaJSPool<Vec<i32>, i32> = OramaJSPool::new(OramaJSPoolConfig {
        pool_config: JSExecutorPoolConfig {
            instances_count_per_code: 2,
            queue_capacity: 10,
            executor_config: JSExecutorConfig {
                allowed_hosts: vec![],
                max_startup_time: std::time::Duration::from_millis(200),
                max_execution_time: std::time::Duration::from_millis(200),
                function_name: "count".to_string(),
                is_async: true,
            },
        },
        // Close the pool if no activity for 60 seconds
        max_idle_time: Duration::from_secs(60),
        // Check every second if there are idle pools to close
        check_interval: Duration::from_secs(1),
    });

    let sleep_in_ms = 10;
    let count_till = 10;

    // Start the counting stream
    let mut receiver = pool
        .execute_stream(CODE_COUNT, vec![count_till, sleep_in_ms])
        .await
        .unwrap();

    // iter on the expected output
    for i in 0..count_till {
        let output = receiver.recv().await.unwrap();
        assert_eq!(output, i + 1);
        println!("counting... {}", output);
    }
    // NB: the receiver is close here!!
    println!("Done!");

    // Close all pools
    pool.close().await.unwrap();
}
```

## License

Licensed under the Affero GPL v3 license. See the LICENSE file for details.
