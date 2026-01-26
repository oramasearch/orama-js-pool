# Orama JS Pool

[![Rust](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml/badge.svg)](https://github.com/oramasearch/orama-js-pool/actions/workflows/ci.yml)

Orama JS Pool provides a pool of JavaScript engines (using the Deno runtime via [deno_core](https://crates.io/crates/deno_core)) for running JavaScript code from Rust. It is designed for high-throughput, parallel, and optionally sandboxed execution of JS functions, supporting both sync and async workflows.

## Quickstart

Here's how to run an async JavaScript function using a pool of JS engines:

```rust
use orama_js_pool::{ExecOption, JSPoolExecutor, JSRunnerError};
use std::time::Duration;

static CODE_ASYNC_SUM: &str = r#"
async function async_sum(a, b) {
    await new Promise(resolve => setTimeout(resolve, 100));
    return a + b
}
export default { async_sum };
"#;

#[tokio::main]
async fn main() -> Result<(), JSRunnerError> {
    // Create a pool with 10 JS engines, running the code above
    let pool = JSPoolExecutor::<Vec<u8>, u8>::new(
        CODE_ASYNC_SUM.to_string(),
        10,                         // number of engines
        None,                       // no http domain restriction on startup
        Duration::from_millis(200), // startup timeout
        true,                       // is_async
        "async_sum".to_string(),    // function name to call
    )
    .await?;

    let params = vec![1, 2];
    let result = pool
        .exec(
            params, // input parameter
            None,   // no stdout stream (set to Some(...) to capture stdout/stderr)
            ExecOption {
                timeout: Duration::from_millis(200), // timeout
                allowed_hosts: None,
            },
        )
        .await?;

    println!("async_sum(1, 2) == {}", result);
    assert_eq!(result, 3);
    Ok(())
}

```

## API Overview

### `JSPoolExecutor`

The main entry point. Manages a pool of JS engines for a given code string. Supports both sync and async JS functions.

- `JSPoolExecutor::<Input, Output>::new(code, pool_size, allowed_hosts, startup_timeout, is_async, function_name)`
- `exec(params, stdout_stream, ExecOption)` — runs the function with the given parameters. The second parameter is an optional stream to receive stdout/stderr output from the JS function.

**Stdout/Stderr Handling:**
You can capture JavaScript `console.log` and `console.error` output by providing a stream (such as a broadcast channel sender) to the `stdout_stream` parameter of the `exec` method. This allows you to handle or redirect JS stdout/stderr as it is produced during execution. See `stream_console` example.

### `ExecOption`

Per-execution configuration:

- `timeout`: Maximum execution time per call
- `allowed_hosts`: Restrict HTTP access for the JS code (optional)

### `JSRunnerError`

All errors (startup, execution, JS exceptions) are reported as `JSRunnerError`.

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
