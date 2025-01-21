use criterion::{criterion_group, criterion_main, Criterion};
use super::TestUtils;
use std::time::Duration;

pub fn command_chain_benchmark(c: &mut Criterion) {
    let chain = TestUtils::create_test_command_chain();
    
    c.bench_function("command_execution", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let mut executor = ChainExecutor::new(chain.clone());
            rt.block_on(executor.execute_next())
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = command_chain_benchmark
}

criterion_main!(benches); 