use std::time::Duration;

use orama_js_pool::{ExecOption, JSPoolExecutor, JSRunnerError};
use tokio::io::{AsyncBufReadExt, BufReader};

static CODE_SUM: &str = r#"
function sum(a, b) {
    return a + b
}
export default { sum };
"#;

#[tokio::main]
async fn main() -> Result<(), JSRunnerError> {
    let _ = tracing_subscriber::fmt::try_init();

    let pool = JSPoolExecutor::<Vec<u8>, u8>::new(
        CODE_SUM.to_string(),
        10,   // 10 executors
        None, // No KV
        None, // No Secrets
        None, // no restriction on http domains
        Duration::from_millis(200),
        false, // is_async
        "sum".to_string(),
    )
    .await?;

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

        let params = vec![a1, a2];
        let output = pool
            .exec(
                params,
                None, // no stdout stream (set to Some(...) to capture stdout/stderr)
                ExecOption {
                    timeout: Duration::from_millis(200),
                    allowed_hosts: None,
                },
            )
            .await?;
        println!("sum({a1}, {a2}) == {output}");
    }

    Ok(())
}
