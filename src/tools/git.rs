// src/tools/git.rs
use super::DevTool;
use super::HealthStatus;
use super::HealthStatusType;
use anyhow::Result;
use std::process::Command;
use crate::config;

pub struct GitTool;

impl GitTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, query: &str, config: &config::Config) -> Result<String> {
        let prompt = format!(
            r#"Convert this Git-related request into a git command: '{}'.
Consider:
- Current repository state
- Branch status
- Staged/unstaged changes
- Safety of operations

Response format:
DANGEROUS: true/false
COMMAND: <command>
EXPLANATION: <why this command was chosen>"#,
            query
        );

        let response = crate::ai::get_command_suggestion(&prompt, config).await?;
        Ok(response.0)
    }
}

impl DevTool for GitTool {
    fn name(&self) -> &'static str {
        "git"
    }

    fn is_available(&self) -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("git").arg("--version").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: if self.is_available() {
                HealthStatusType::Healthy
            } else {
                HealthStatusType::Unhealthy
            },
            message: self.version()?.to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn generate_command(&self, query: &str) -> Result<String> {
        Ok(format!("git {}", query))
    }

    fn validate_command(&self, command: &str) -> Result<bool> {
        Ok(!command.contains("--force") && !command.contains("-f"))
    }

    fn explain_command(&self, command: &str) -> Result<String> {
        let prompt = format!(
            r#"Explain this git command in clear, concise bullet points:
Command: {}

Consider:
- What changes will occur
- Impact on branches/history
- Safety of the operation
- Best practices"#,
            command
        );
        Ok(prompt) // The actual AI call will be made in execute()
    }
}
