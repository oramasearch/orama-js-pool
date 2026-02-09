use orama_js_pool::{ExecOptions, Pool, RuntimeError};
use std::time::Duration;

static CODE_ASYNC_SUM: &str = r#"
async function async_sum(a, b) {
    await new Promise(resolve => setTimeout(resolve, 100));
    return a + b
}
export default { async_sum };
"#;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    // Create a pool with 10 JS workers, with the module loaded
    let pool = Pool::builder()
        .max_size(10) // number of workers in the pool
        .with_evaluation_timeout(Duration::from_millis(200)) // module load timeout
        .add_module("async_calculator", CODE_ASYNC_SUM.to_string())
        .build()
        .await?;

    let params = vec![1, 2];
    let result: u8 = pool
        .exec(
            "async_calculator", // module name
            "async_sum",        // function name
            &params,            // input parameters
            ExecOptions::new().with_timeout(Duration::from_millis(200)),
        )
        .await?;

    println!("async_sum(1, 2) == {result}");
    assert_eq!(result, 3);
    Ok(())
}
