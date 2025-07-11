use std::time::Duration;

use orama_js_pool::{ExecOption, JSPoolExecutor, JSRunnerError};

// Ths code is generated building `open_ai_js_example` code
// and replace `./openai.js` content with the `open_ai_js_example/dist/openai.js` one
static CODE: &str = include_str!("./openai.js");

#[tokio::main]
async fn main() -> Result<(), JSRunnerError> {
    let open_api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable is not set");

    let pool = JSPoolExecutor::<Vec<String>, String>::new(
        CODE.to_string(),
        10,   // 10 executors
        None, // no http domain restriction on startup
        Duration::from_millis(200),
        true, // is_async
        "getResponse".to_string(),
    )
    .await?;

    let params = vec![
        open_api_key,
        "gpt-4o".to_string(),
        "Say this is a test".to_string(),
    ];

    let result = pool
        .exec(
            params,
            None, // no stdout stream (set to Some(...) to capture stdout/stderr)
            ExecOption {
                timeout: Duration::from_millis(5_000),
                allowed_hosts: None,
            },
        )
        .await?;

    // Output:
    // ```
    // This is a test.
    // ```
    println!("{}", result);

    Ok(())
}
