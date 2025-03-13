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
