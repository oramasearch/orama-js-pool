use std::time::Duration;

use orama_js_pool::{ExecOptions, Pool, RuntimeError};
use tokio::io::{AsyncBufReadExt, BufReader};

static CODE_SUM: &str = r#"
function sum(a, b) {
    return a + b
}
export default { sum };
"#;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let _ = tracing_subscriber::fmt::try_init();

    let pool = Pool::builder()
        .max_size(10) // 10 workers in the pool
        .with_evaluation_timeout(Duration::from_millis(200))
        .add_module("calculator", CODE_SUM.to_string())
        .build()?;

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
        let output: u8 = pool
            .exec(
                "calculator", // module name
                "sum",        // function name
                params,
                ExecOptions::new().with_timeout(Duration::from_millis(200)),
            )
            .await?;
        println!("sum({a1}, {a2}) == {output}");
    }

    Ok(())
}
