#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create Rust project structure
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
            name = "test_project"
            version = "0.1.0"
            
            [dependencies]
            serde = "1.0"
            "#,
        )
        .unwrap();

        // Create Node.js project structure
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{
                "name": "test-project",
                "version": "1.0.0",
                "dependencies": {
                    "react": "^17.0.0"
                }
            }"#,
        )
        .unwrap();

        // Create Python project structure
        fs::write(
            temp_dir.path().join("pyproject.toml"),
            r#"[tool.poetry]
            name = "test-project"
            version = "0.1.0"
            
            [build-system]
            requires = ["poetry-core>=1.0.0"]
            build-backend = "poetry.core.masonry.api"
            "#,
        )
        .unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_framework_detection() {
        let temp_dir = setup_test_project();
        let detector = FrameworkDetector::new();

        let frameworks = detector.detect(temp_dir.path()).await.unwrap();

        // Verify Rust framework detection
        assert!(frameworks.iter().any(|f| f.name == "Rust/Cargo"));

        // Verify Node.js framework detection
        assert!(frameworks.iter().any(|f| f.name == "Node.js"));

        // Verify Python framework detection
        assert!(frameworks.iter().any(|f| f.name == "Poetry"));
    }

    #[tokio::test]
    async fn test_framework_versions() {
        let temp_dir = setup_test_project();
        let detector = FrameworkDetector::new();

        let frameworks = detector.detect(temp_dir.path()).await.unwrap();

        // Check version detection
        for framework in frameworks {
            match framework.name.as_str() {
                "Rust/Cargo" => {
                    assert_eq!(framework.version, Some("0.1.0".to_string()));
                }
                "Node.js" => {
                    assert_eq!(framework.version, Some("1.0.0".to_string()));
                }
                "Poetry" => {
                    assert_eq!(framework.version, Some("0.1.0".to_string()));
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_confidence_levels() {
        let temp_dir = setup_test_project();
        let detector = FrameworkDetector::new();

        let frameworks = detector.detect(temp_dir.path()).await.unwrap();

        for framework in frameworks {
            assert!(
                framework.confidence >= 0.5,
                "Confidence should be at least 0.5, got {}",
                framework.confidence
            );
            assert!(
                framework.confidence <= 1.0,
                "Confidence should not exceed 1.0, got {}",
                framework.confidence
            );
        }
    }
}
