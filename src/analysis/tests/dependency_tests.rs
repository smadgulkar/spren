#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        // Create Rust dependencies
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
            name = "test_project"
            version = "0.1.0"
            
            [dependencies]
            serde = "1.0"
            tokio = { version = "1.0", features = ["full"] }

            [dev-dependencies]
            mockito = "1.0"
            "#,
        ).unwrap();

        // Create Node.js dependencies
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{
                "name": "test-project",
                "version": "1.0.0",
                "dependencies": {
                    "react": "^17.0.0",
                    "express": "^4.17.1"
                },
                "devDependencies": {
                    "jest": "^27.0.0"
                }
            }"#,
        ).unwrap();

        // Create Python dependencies
        fs::write(
            temp_dir.path().join("requirements.txt"),
            "requests==2.26.0\nflask>=2.0.0\n",
        ).unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_rust_dependencies() {
        let temp_dir = setup_test_project();
        let analyzer = DependencyAnalyzer::new();
        
        let deps = analyzer.analyze(temp_dir.path()).await.unwrap();
        
        // Check Rust dependencies
        let rust_deps: Vec<_> = deps.iter()
            .filter(|d| d.source == "crates.io")
            .collect();
        
        assert!(rust_deps.iter().any(|d| d.name == "serde" && d.version == "1.0"));
        assert!(rust_deps.iter().any(|d| d.name == "tokio"));
        assert!(rust_deps.iter().any(|d| d.name == "mockito" && d.is_dev));
    }

    #[tokio::test]
    async fn test_node_dependencies() {
        let temp_dir = setup_test_project();
        let analyzer = DependencyAnalyzer::new();
        
        let deps = analyzer.analyze(temp_dir.path()).await.unwrap();
        
        // Check Node.js dependencies
        let node_deps: Vec<_> = deps.iter()
            .filter(|d| d.source == "npm")
            .collect();
        
        assert!(node_deps.iter().any(|d| d.name == "react" && !d.is_dev));
        assert!(node_deps.iter().any(|d| d.name == "express" && !d.is_dev));
        assert!(node_deps.iter().any(|d| d.name == "jest" && d.is_dev));
    }

    #[tokio::test]
    async fn test_python_dependencies() {
        let temp_dir = setup_test_project();
        let analyzer = DependencyAnalyzer::new();
        
        let deps = analyzer.analyze(temp_dir.path()).await.unwrap();
        
        // Check Python dependencies
        let python_deps: Vec<_> = deps.iter()
            .filter(|d| d.source == "pip")
            .collect();
        
        assert!(python_deps.iter().any(|d| d.name == "requests"));
        assert!(python_deps.iter().any(|d| d.name == "flask"));
    }
} 