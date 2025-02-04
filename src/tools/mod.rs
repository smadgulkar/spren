// src/tools/mod.rs
mod docker;
mod git;
mod kubernetes;

pub use docker::DockerTool;
pub use git::GitTool;
pub use kubernetes::KubernetesTool;

use crate::config;
use crate::shell::ShellType;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerConfig {
    pub enabled: bool,
    pub compose_version: Option<String>,
    pub default_registry: Option<String>,
    pub cleanup_policy: CleanupPolicy,
    pub health_check_interval: u64,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub enabled: bool,
    pub current_context: Option<String>,
    pub namespaces: Vec<String>,
    pub resource_quotas: bool,
    pub auto_cleanup: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitConfig {
    pub enabled: bool,
    pub default_branch: String,
    pub commit_template: Option<String>,
    pub auto_fetch_interval: u64,
    pub protected_branches: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupPolicy {
    pub max_container_age: u64,
    pub max_stopped_containers: u32,
    pub max_dangling_images: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory: String,
    pub max_cpu: String,
    pub max_containers: u32,
}

#[derive(Debug)]
pub enum HealthStatusType {
    Healthy,
    Unhealthy,
}

#[derive(Debug)]
pub struct HealthStatus {
    pub status: HealthStatusType,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub trait DevTool {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn version(&self) -> Result<String>;
    fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: if self.is_available() {
                HealthStatusType::Healthy
            } else {
                HealthStatusType::Unhealthy
            },
            message: String::new(),
            timestamp: chrono::Utc::now(),
        })
    }
    fn generate_command(&self, query: &str) -> Result<String> {
        Ok(format!("{} {}", self.name(), query))
    }
    fn validate_command(&self, _command: &str) -> Result<bool> {
        Ok(true)
    }
    fn explain_command(&self, command: &str) -> Result<String> {
        Ok(format!("Explanation for: {}", command))
    }
}

pub struct ToolsConfig {
    pub docker_enabled: bool,
    pub kubernetes_enabled: bool,
    pub git_enabled: bool,
}

impl ToolsConfig {
    pub fn detect() -> Result<Self> {
        Ok(Self {
            docker_enabled: DockerTool::new().is_available(),
            kubernetes_enabled: KubernetesTool::new().is_available(),
            git_enabled: GitTool::new().is_available(),
        })
    }

    pub async fn process_query(&self, query: &str, config: &config::Config) -> Result<String> {
        let shell_type = ShellType::detect();

        // First try to determine if this is a tool-specific query
        let tool_prompt = format!(
            r#"Is this query for a specific tool? Query: '{}'
Available tools:
- Git (available: {})
- Docker (available: {})
- Kubernetes (available: {})

Response format:
TOOL: git|docker|kubernetes|shell
REASON: <why this tool was chosen>"#,
            query, self.git_enabled, self.docker_enabled, self.kubernetes_enabled
        );

        let response = crate::ai::get_command_suggestion(&tool_prompt, config).await?;
        let (tool_name, _) = parse_tool_response(&response.0)?;

        // Generate the appropriate command
        match tool_name {
            "git" if self.git_enabled => GitTool::new().execute(query, config).await,
            "docker" if self.docker_enabled => DockerTool::new().execute(query, config).await,
            "kubernetes" if self.kubernetes_enabled => {
                KubernetesTool::new().execute(query, config).await
            }
            _ => {
                let shell_prompt = format!(
                    r#"Convert this query into a {} command: '{}'
Consider:
- Current shell: {}
- Available tools: git={}, docker={}, kubernetes={}
- Use only commands that work in this shell
- No PowerShell-specific commands for CMD
- No CMD-specific commands for PowerShell

Response format:
DANGEROUS: true/false
COMMAND: <command>
EXPLANATION: <why this command was chosen>"#,
                    shell_type.get_shell_name(),
                    query,
                    shell_type.get_shell_name(),
                    self.git_enabled,
                    self.docker_enabled,
                    self.kubernetes_enabled
                );

                crate::ai::get_command_suggestion(&shell_prompt, config)
                    .await
                    .map(|r| r.0)
            }
        }
    }
}

fn parse_tool_response(response: &str) -> Result<(&str, String)> {
    // Parse the response more flexibly
    let response = response.trim().to_lowercase();

    // Try to find tool type in the response
    let tool = if response.contains("docker") {
        "docker"
    } else if response.contains("git") {
        "git"
    } else if response.contains("kubernetes") || response.contains("kubectl") {
        "kubernetes"
    } else {
        "shell"
    };

    // Use the original query if we can't parse it properly
    let query = if let Some(idx) = response.find("query:") {
        response[idx..]
            .trim_start_matches("query:")
            .trim()
            .to_string()
    } else {
        response.trim().to_string()
    };

    Ok((tool, query))
}
