use criterion::{criterion_group, criterion_main, Criterion};
use super::TestUtils;
use std::time::Duration;
use crate::analysis::ProjectAnalyzer;
use anyhow::Result;
use std::time::Instant;

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

#[tokio::test]
async fn test_analysis_performance() -> Result<()> {
    let utils = TestUtils::new()?;
    let project_dir = utils.create_test_project()?;
    
    let analyzer = ProjectAnalyzer::new(&project_dir);
    
    let start = Instant::now();
    let analysis = analyzer.analyze().await?;
    let duration = start.elapsed();

    // Basic analysis should complete within 1 second
    assert!(duration.as_secs() < 1, "Analysis took too long: {:?}", duration);
    
    // Verify minimum analysis requirements
    assert!(!analysis.languages.is_empty(), "No languages detected");
    assert!(!analysis.frameworks.is_empty(), "No frameworks detected");
    assert!(!analysis.dependencies.is_empty(), "No dependencies detected");

    Ok(())
}

#[tokio::test]
async fn test_large_project_performance() -> Result<()> {
    let utils = TestUtils::new()?;
    let project_dir = utils.create_test_project()?;
    
    // Create a large number of files
    for i in 0..100 {
        std::fs::write(
            project_dir.join(format!("file_{}.rs", i)),
            "fn main() { println!(\"Hello\"); }",
        )?;
    }
    
    let analyzer = ProjectAnalyzer::new(&project_dir);
    let start = Instant::now();
    let _analysis = analyzer.analyze().await?;
    let duration = start.elapsed();

    // Large project analysis should complete within 5 seconds
    assert!(duration.as_secs() < 5, "Large project analysis took too long: {:?}", duration);

    Ok(())
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = command_chain_benchmark
}

criterion_main!(benches); 