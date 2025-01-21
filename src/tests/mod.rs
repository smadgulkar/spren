use crate::ai::{CommandChain, CommandStep, ResourceImpact};
use crate::shell::ShellType;
use std::time::Duration;

mod command_chain;
mod error_handling;
mod integration;
mod performance;

use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;

// Test utilities and helpers
pub(crate) struct TestUtils {
    temp_dir: TempDir,
}

impl TestUtils {
    pub fn new() -> Result<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
        })
    }

    pub fn create_test_project(&self) -> Result<PathBuf> {
        let project_dir = self.temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_dir)?;

        // Create test files for different project types
        self.create_rust_project(&project_dir)?;
        self.create_node_project(&project_dir)?;
        self.create_python_project(&project_dir)?;

        Ok(project_dir)
    }

    fn create_rust_project(&self, base_dir: &PathBuf) -> Result<()> {
        let cargo_toml = base_dir.join("Cargo.toml");
        std::fs::write(
            cargo_toml,
            r#"[package]
name = "test_project"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
mockito = "1.0""#,
        )?;
        Ok(())
    }

    fn create_node_project(&self, base_dir: &PathBuf) -> Result<()> {
        let package_json = base_dir.join("package.json");
        std::fs::write(
            package_json,
            r#"{
    "name": "test-project",
    "version": "1.0.0",
    "dependencies": {
        "express": "^4.17.1"
    },
    "devDependencies": {
        "jest": "^27.0.0"
    }
}"#,
        )?;
        Ok(())
    }

    fn create_python_project(&self, base_dir: &PathBuf) -> Result<()> {
        let requirements_txt = base_dir.join("requirements.txt");
        std::fs::write(requirements_txt, "requests==2.26.0\nflask>=2.0.0\n")?;
        Ok(())
    }

    pub fn create_test_command_chain() -> CommandChain {
        CommandChain {
            steps: vec![CommandStep {
                command: "ls -la".to_string(),
                explanation: "List directory contents".to_string(),
                is_dangerous: false,
                impact: ResourceImpact {
                    cpu_usage: 0.1,
                    memory_usage: 5.0,
                    disk_usage: 0.0,
                    network_usage: 0.0,
                    estimated_duration: Duration::from_secs(1),
                },
                rollback_command: None,
            }],
            total_impact: ResourceImpact {
                cpu_usage: 0.1,
                memory_usage: 5.0,
                disk_usage: 0.0,
                network_usage: 0.0,
                estimated_duration: Duration::from_secs(1),
            },
            explanation: "Test command chain".to_string(),
        }
    }
}

#[cfg(test)]
mod test_helpers {
    use std::sync::Once;
    use test_log::test;

    static INIT: Once = Once::new();

    pub fn initialize() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    #[test]
    fn test_test_infrastructure() {
        initialize();

        // Test project creation
        let utils = super::TestUtils::new().unwrap();
        let project_dir = utils.create_test_project().unwrap();

        // Verify Rust project files
        assert!(project_dir.join("Cargo.toml").exists());

        // Verify Node.js project files
        assert!(project_dir.join("package.json").exists());

        // Verify Python project files
        assert!(project_dir.join("requirements.txt").exists());

        // Test cleanup (tempdir should clean up automatically)
    }

    #[test]
    fn test_command_chain_creation() {
        let chain = super::TestUtils::create_test_command_chain();
        assert!(!chain.steps.is_empty());
        assert!(chain.steps[0].command.len() > 0);
        assert!(!chain.steps[0].is_dangerous);
        assert!(chain.total_impact.cpu_usage > 0.0);
    }
}
