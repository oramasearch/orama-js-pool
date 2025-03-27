use std::time::Duration;

use orama_js_pool::{JSExecutorConfig, JSExecutorPoolConfig, OramaJSPool, OramaJSPoolConfig};
use tokio::io::{AsyncBufReadExt, BufReader};

static CODE_SUM: &str = r#"
function sum(a, b) {
    return a + b
}
export default { sum };
"#;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();

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
        max_idle_time: Duration::from_secs(5),
        // Check every second if there are idle pools to close
        check_interval: Duration::from_secs(1),
    });

    let stdin = tokio::io::stdin();
    let mut c = BufReader::new(stdin);
    let mut buff = String::new();

    while c.read_line(&mut buff).await.is_ok() {
        if buff == "exit\n" {
            break;
        }

        let (a1, a2) = match buff.split_once(" ") {
            Some((a1, a2)) => (a1, a2),
            None => {
                println!(
                    "Invalid input. Please provide two numbers separated by a space or 'exit'"
                );
                buff.clear();
                continue;
            }
        };
        let a1 = a1.parse::<u8>().unwrap();
        let a2 = a2.trim().parse::<u8>().unwrap();
        buff.clear();

        let output: u8 = pool.execute(CODE_SUM, vec![a1, a2]).await.unwrap();
        println!("sum({a1}, {a2}) == {}", output);
    }

    // Close all pools
    pool.close().await.unwrap();
}
