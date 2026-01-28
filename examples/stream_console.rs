use orama_js_pool::{ExecOptions, OutputChannel, Pool, RuntimeError};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

static CODE_LOG: &str = r#"
function log_and_error(a) {
    console.log('console.log:', a);
    console.error('console.error:', a);
    return Object.keys(a).length;
}
export default { log_and_error };
"#;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let pool = Pool::builder()
        .max_size(2) // 2 workers in the pool
        .with_evaluation_timeout(Duration::from_millis(200))
        .add_module("logger", CODE_LOG.to_string())
        .build()?;

    let (sender, mut receiver) = broadcast::channel::<(OutputChannel, String)>(16);
    let input = json!({
        "a": "foo",
        "b": 42,
    });

    // Spawn a task to print streamed output as it arrives
    let print_task = tokio::spawn(async move {
        while let Ok((channel, msg)) = receiver.recv().await {
            match channel {
                OutputChannel::StdOut => print!("[JS stdout] {msg}"),
                OutputChannel::StdErr => eprint!("[JS stderr] {msg}"),
            }
        }
    });

    let result: usize = pool
        .exec(
            "logger",        // module name
            "log_and_error", // function name
            input.clone(),
            ExecOptions::new()
                .with_timeout(Duration::from_millis(500))
                .with_stdout_sender(Arc::new(sender)),
        )
        .await?;

    println!("Returned value: {result}");
    // Give the print task a moment to flush output
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(print_task); // End the print task
    Ok(())
}
