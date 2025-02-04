// src/tools/docker.rs
use super::DevTool;
use super::HealthStatus;
use super::HealthStatusType;
use crate::config;
use anyhow::Result;
use std::process::Command;

pub struct DockerTool;

impl DockerTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, query: &str, config: &config::Config) -> Result<String> {
        let prompt = format!(
            r#"Convert this Docker-related query into a proper command: '{}'.
Consider:
- Current context and running containers
- Security implications
- Resource usage
- Best practices

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

impl DevTool for DockerTool {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn is_available(&self) -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("docker").arg("--version").output()?;
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
        Ok(format!("docker {}", query))
    }

    fn validate_command(&self, command: &str) -> Result<bool> {
        Ok(!command.contains("--privileged"))
    }

    fn explain_command(&self, command: &str) -> Result<String> {
        let prompt = format!(
            r#"Explain this Docker command in clear, concise bullet points:
Command: {}

Consider:
- What each flag and argument does
- Potential impact on the system
- Security implications
- Resource usage"#,
            command
        );
        Ok(prompt) // The actual AI call will be made in execute()
    }
}
