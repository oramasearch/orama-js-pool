# Orama JS Pool

[![Rust](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml/badge.svg)](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml)

Orama JS Pool provides a pool of JavaScript engines (using the Deno runtime via [deno_core](https://crates.io/crates/deno_core)) for running JavaScript code from Rust. It is designed for high-throughput, parallel, and optionally sandboxed execution of JS functions, supporting both sync and async workflows.

## Quickstart

Here's how to run an async JavaScript function using a pool of JS engines:

```rust
use orama_js_pool::{ExecOptions, Pool, RuntimeError};
use std::time::Duration;

static CODE_ASYNC_SUM: &str = r#"
async function async_sum(a, b) {
    await new Promise(resolve => setTimeout(resolve, 100));
    return a + b
}
export default { async_sum };
"#;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    // Create a pool with 10 JS workers, with the module loaded
    let pool = Pool::builder()
        .max_size(10) // number of workers in the pool
        .with_evaluation_timeout(Duration::from_millis(200)) // module load timeout
        .add_module("async_calculator", CODE_ASYNC_SUM.to_string())
        .build()
        .await?;

    let params = vec![1, 2];
    let result: u8 = pool
        .exec(
            "async_calculator", // module name
            "async_sum",        // function name
            params,             // input parameters
            ExecOptions::new().with_timeout(Duration::from_millis(200)),
        )
        .await?;

    println!("async_sum(1, 2) == {result}");
    assert_eq!(result, 3);
    Ok(())
}

```

## API Overview

### `Pool`

Main entry point. Manages a pool of JS workers, each with loaded modules.

- `Pool::builder()` — creates a new pool builder
- `PoolBuilder::max_size(n)` — sets worker pool size
- `PoolBuilder::with_evaluation_timeout(duration)` — module load timeout
- `PoolBuilder::add_module(name, code)` — loads a module into all workers
- `exec(module_name, function_name, params, ExecOptions)` — executes a function

### `ExecOptions`

Per-execution configuration:

- `with_timeout(duration)`: Maximum execution time
- `with_allowed_hosts(hosts)`: Restrict HTTP access
- `with_stdout_stream(sender)`: Capture console.log/error output

### `RuntimeError`

All errors (startup, execution, JS exceptions) are reported as `RuntimeError`.

## Features

- **Parallel execution**: Multiple requests handled concurrently
- **Async and sync JS support**: Run both types of JS functions
- **Sandboxing**: Restrict network access via `allowed_hosts`
- **Timeouts**: Prevent runaway scripts
- **Typed input/output**: Use Rust types for parameters and results (via serde)

## Example: Streaming (if supported)

If your JS function is an async generator, you can use streaming APIs (see crate docs for details).

## License

Licensed under the Affero GPL v3 license. See the LICENSE file for details.
