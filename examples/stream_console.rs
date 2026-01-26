use orama_js_pool::{ExecOption, JSPoolExecutor, JSRunnerError, OutputChannel};
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
async fn main() -> Result<(), JSRunnerError> {
    let pool = JSPoolExecutor::<serde_json::Value, usize>::new(
        CODE_LOG.to_string(),
        2,    // 2 executors
        None, // no http domain restriction
        Duration::from_millis(200),
        false, // not async
        "log_and_error".to_string(),
    )
    .await?;

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

    let result = pool
        .exec(
            input.clone(),
            Some(Arc::new(sender)),
            ExecOption {
                timeout: Duration::from_millis(500),
                allowed_hosts: None,
            },
        )
        .await?;

    println!("Returned value: {result}");
    // Give the print task a moment to flush output
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(print_task); // End the print task
    Ok(())
}
