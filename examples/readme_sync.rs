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
