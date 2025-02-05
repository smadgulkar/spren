// src/tools/mod.rs
use crate::config;
use crate::shell::ShellType;
use anyhow::Result;
use chrono;
use std::collections::HashMap;

// Module declarations
mod code_generator;
mod docker;
mod git;
mod kubernetes;

// Public exports
pub use code_generator::CodeGeneratorTool;
pub use docker::DockerTool;
pub use git::GitTool;
pub use kubernetes::KubernetesTool;

/// Represents the health status of a tool
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatusType {
    Healthy,
    Unhealthy,
}

/// Contains health check information for a tool
#[derive(Debug)]
pub struct HealthStatus {
    pub status: HealthStatusType,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Core trait that all development tools must implement
pub trait DevTool {
    /// Returns the name of the tool
    fn name(&self) -> &'static str;

    /// Checks if the tool is available in the current environment
    fn is_available(&self) -> bool;

    /// Returns the version of the tool
    fn version(&self) -> Result<String>;

    /// Performs a health check on the tool
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

    /// Generates a command string from a query
    fn generate_command(&self, query: &str) -> Result<String>;

    /// Validates if a command is safe to execute
    fn validate_command(&self, _command: &str) -> Result<bool> {
        Ok(true)
    }

    /// Provides an explanation for a command
    fn explain_command(&self, command: &str) -> Result<String> {
        Ok(format!("Explanation for: {}", command))
    }
}

/// Configuration for available tools in the environment
#[derive(Debug)]
pub struct ToolsConfig {
    pub docker_enabled: bool,
    pub kubernetes_enabled: bool,
    pub git_enabled: bool,
    pub code_generator_enabled: bool,
}

impl ToolsConfig {
    /// Detects which tools are available in the current environment
    pub fn detect() -> Result<Self> {
        Ok(Self {
            docker_enabled: DockerTool::new().is_available(),
            kubernetes_enabled: KubernetesTool::new().is_available(),
            git_enabled: GitTool::new().is_available(),
            code_generator_enabled: CodeGeneratorTool::new().is_available(),
        })
    }

    /// Processes a user query and routes it to the appropriate tool
    pub async fn process_query(&self, query: &str, config: &config::Config) -> Result<String> {
        // Get the current shell type for proper command formatting
        let shell_type = ShellType::detect();

        // Determine which tool should handle the query
        let response = self.determine_tool(query, config).await?;
        let (tool_name, _) = parse_tool_response(&response.0)?;

        match tool_name {
            "git" if self.git_enabled => GitTool::new().execute(query, config).await,
            "docker" if self.docker_enabled => DockerTool::new().execute(query, config).await,
            "kubernetes" if self.kubernetes_enabled => {
                KubernetesTool::new().execute(query, config).await
            }
            "code-generator" if self.code_generator_enabled => {
                CodeGeneratorTool::new().execute(query, config).await
            }
            _ => {
                // Default to shell commands if no specific tool matches
                self.process_shell_command(query, &shell_type, config).await
            }
        }
    }

    /// Determines which tool should handle a query
    async fn determine_tool(&self, query: &str, config: &config::Config) -> Result<(String, bool)> {
        let tool_prompt = format!(
            r#"Is this query for a specific tool? Query: '{}'
Available tools:
- Git (available: {})
- Docker (available: {})
- Kubernetes (available: {})
- Code Generator (available: {})

Response format:
TOOL: git|docker|kubernetes|code-generator|shell
REASON: <why this tool was chosen>"#,
            query,
            self.git_enabled,
            self.docker_enabled,
            self.kubernetes_enabled,
            self.code_generator_enabled
        );

        crate::ai::get_command_suggestion(&tool_prompt, config).await
    }

    /// Processes a shell command when no specific tool matches
    async fn process_shell_command(
        &self,
        query: &str,
        shell_type: &ShellType,
        config: &config::Config,
    ) -> Result<String> {
        let shell_prompt = format!(
            r#"Convert this query into a {} command: '{}'
Consider:
- Current shell: {}
- Available tools: git={}, docker={}, kubernetes={}, code-generator={}
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
            self.kubernetes_enabled,
            self.code_generator_enabled
        );

        crate::ai::get_command_suggestion(&shell_prompt, config)
            .await
            .map(|r| r.0)
    }
}

/// Parses the AI response to determine which tool should handle the query
fn parse_tool_response(response: &str) -> Result<(&str, String)> {
    let response = response.trim().to_lowercase();

    // Determine the appropriate tool based on response content
    let tool = if response.contains("docker") {
        "docker"
    } else if response.contains("git") {
        "git"
    } else if response.contains("kubernetes") || response.contains("kubectl") {
        "kubernetes"
    } else if response.contains("code-generator") || response.contains("generate") {
        "code-generator"
    } else {
        "shell"
    };

    // Extract the original query or use the full response
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
