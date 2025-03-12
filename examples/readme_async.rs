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
