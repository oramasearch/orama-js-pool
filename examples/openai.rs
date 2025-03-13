use std::time::Duration;

use orama_js_pool::{JSExecutorConfig, JSExecutorPoolConfig, OramaJSPool, OramaJSPoolConfig};

// Ths code is generated building `open_ai_js_example` code
// and replace `./openai.js` content with the `open_ai_js_example/dist/openai.js` one
static CODE: &str = include_str!("./openai.js");

#[tokio::main]
async fn main() {
    let open_api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable is not set");

    let pool = OramaJSPool::new(OramaJSPoolConfig {
        pool_config: JSExecutorPoolConfig {
            instances_count_per_code: 1,
            queue_capacity: 10,
            executor_config: JSExecutorConfig {
                allowed_hosts: vec!["api.openai.com".to_string()],
                max_startup_time: Duration::from_millis(200),
                max_execution_time: Duration::from_millis(5_000),
                function_name: "getResponse".to_string(),
                is_async: true,
            },
        },
        max_idle_time: Duration::from_secs(60),
        check_interval: Duration::from_secs(1),
    });

    let output: String = pool
        .execute(
            CODE,
            vec![
                open_api_key,
                "gpt-4o".to_string(),
                "Say this is a test".to_string(),
            ],
        )
        .await
        .unwrap();

    // Output:
    // ```
    // This is a test.
    // ```
    println!("{}", output);

    pool.close().await.unwrap();
}
