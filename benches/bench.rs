use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use orama_js_pool::{DomainPermission, ExecOptions, Pool, Worker};
use std::time::Duration;
use tokio::runtime::Runtime as TokioRuntime;

static SIMPLE_CODE: &str = r#"
function add(a, b) {
    return a + b;
}
export default { add };
"#;

fn bench_worker_init(c: &mut Criterion) {
    let rt = TokioRuntime::new().unwrap();

    c.bench_function("worker/init", |b| {
        b.to_async(&rt).iter(|| async {
            let worker = Worker::builder()
                .with_evaluation_timeout(Duration::from_secs(5))
                .with_domain_permission(DomainPermission::AllowAll)
                .add_module("benchmark", SIMPLE_CODE.to_string())
                .build()
                .await
                .unwrap();
            black_box(worker);
        });
    });
}

fn bench_worker_exec(c: &mut Criterion) {
    let rt = TokioRuntime::new().unwrap();

    c.bench_function("worker/exec", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async {
                let mut worker = Worker::builder()
                    .with_evaluation_timeout(Duration::from_secs(5))
                    .with_domain_permission(DomainPermission::AllowAll)
                    .add_module("benchmark", SIMPLE_CODE.to_string())
                    .build()
                    .await
                    .unwrap();

                let start = std::time::Instant::now();
                for _i in 0..iters {
                    let result: i32 = worker
                        .exec(
                            "benchmark",
                            "add",
                            &vec![black_box(5), black_box(10)],
                            ExecOptions::new().with_timeout(Duration::from_secs(1)),
                        )
                        .await
                        .unwrap();
                    black_box(result);
                }
                start.elapsed()
            })
        });
    });
}

fn bench_pool_init(c: &mut Criterion) {
    let rt = TokioRuntime::new().unwrap();

    c.bench_function("pool/initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = Pool::builder()
                .max_size(5)
                .with_evaluation_timeout(Duration::from_secs(5))
                .with_domain_permission(DomainPermission::AllowAll)
                .add_module("benchmark", SIMPLE_CODE.to_string())
                .build()
                .await
                .unwrap();
            black_box(pool);
        });
    });
}

fn bench_pool_execution(c: &mut Criterion) {
    let rt = TokioRuntime::new().unwrap();

    c.bench_function("pool/exec_add", |b| {
        b.iter_custom(|iters| {
            rt.block_on(async {
                let pool = Pool::builder()
                    .max_size(5)
                    .with_evaluation_timeout(Duration::from_secs(5))
                    .with_domain_permission(DomainPermission::AllowAll)
                    .add_module("benchmark", SIMPLE_CODE.to_string())
                    .build()
                    .await
                    .unwrap();

                let start = std::time::Instant::now();
                for _i in 0..iters {
                    let result: i32 = pool
                        .exec(
                            "benchmark",
                            "add",
                            &vec![black_box(5), black_box(10)],
                            ExecOptions::new().with_timeout(Duration::from_secs(1)),
                        )
                        .await
                        .unwrap();
                    black_box(result);
                }
                start.elapsed()
            })
        });
    });
}

criterion_group!(
    benches,
    bench_worker_init,
    bench_worker_exec,
    bench_pool_init,
    bench_pool_execution
);
criterion_main!(benches);
