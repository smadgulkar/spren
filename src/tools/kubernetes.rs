// src/tools/kubernetes.rs
use super::DevTool;
use super::HealthStatus;
use super::HealthStatusType;
use crate::config;
use anyhow::Result;
use std::process::Command;

pub struct KubernetesTool;

impl KubernetesTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, query: &str, config: &config::Config) -> Result<String> {
        let prompt = self.get_command_prompt(query);
        let response = crate::ai::get_command_suggestion(&prompt, config).await?;
        Ok(response.0)
    }

    fn get_command_prompt(&self, query: &str) -> String {
        format!(
            r#"Convert this Kubernetes-related query into a kubectl command: '{}'.
Consider:
- Current context and namespace
- Resource types and names
- Cluster state and health
- Security best practices

Response format:
DANGEROUS: true/false
COMMAND: <command>
EXPLANATION: <why this command was chosen>"#,
            query
        )
    }
}

impl DevTool for KubernetesTool {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

    fn is_available(&self) -> bool {
        Command::new("kubectl")
            .arg("version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("kubectl").arg("version").output()?;
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
        Ok(format!("kubectl {}", query))
    }

    fn validate_command(&self, command: &str) -> Result<bool> {
        Ok(!command.contains("--force") && !command.contains("-f"))
    }

    fn explain_command(&self, command: &str) -> Result<String> {
        let prompt = format!(
            r#"Explain this kubectl command in clear, concise bullet points:
Command: {}

Consider:
- Resource types affected
- Namespace impact
- Cluster state changes
- Security implications"#,
            command
        );
        Ok(prompt) // The actual AI call will be made in execute()
    }
}
