//src/tools/git.rs
use super::DevTool;
use super::HealthStatus;
use super::HealthStatusType;
use anyhow::Result;
use std::process::Command;
use std::collections::HashMap;
use crate::config;

pub struct GitTool;

impl GitTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, query: &str, config: &config::Config) -> Result<String> {
        // For commit operations, handle the flow directly
        if query.to_lowercase().contains("commit") {
            // First check status
            let status_output = Command::new("git")
                .args(["status", "--porcelain"])
                .output()?;
            
            let status = String::from_utf8_lossy(&status_output.stdout);
            if status.is_empty() {
                return Ok("No changes to commit".to_string());
            }

            // Analyze changes
            let mut files_by_type: HashMap<String, Vec<String>> = HashMap::new();
            for line in status.lines() {
                if line.len() < 3 { continue; }
                let file = line[3..].to_string();
                let ext = std::path::Path::new(&file)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("other")
                    .to_string();
                files_by_type.entry(ext).or_default().push(file);
            }

            // Stage changes if needed
            let add_result = Command::new("git")
                .args(["add", "."])
                .output()?;
            
            if !add_result.status.success() {
                return Ok(format!("Error staging files: {}", 
                    String::from_utf8_lossy(&add_result.stderr)));
            }

            // Generate commit message
            let msg = if files_by_type.contains_key("rs") {
                let rust_files: Vec<_> = files_by_type.get("rs").unwrap().iter()
                    .map(|f| f.split('/').last().unwrap_or(f))
                    .collect();
                format!("feat: Update Rust code in {}", rust_files.join(", "))
            } else if files_by_type.iter().any(|(k, _)| k == "md" || k == "txt") {
                "docs: Update documentation".to_string()
            } else if files_by_type.contains_key("toml") {
                "chore: Update project configuration".to_string()
            } else {
                "chore: Update project files".to_string()
            };

            // Add detailed changes to commit message
            let mut full_msg = msg;
            full_msg.push_str("\n\nChanged files:");
            for line in status.lines() {
                if line.len() > 3 {
                    full_msg.push_str(&format!("\n- {}", &line[3..]));
                }
            }

            // Perform commit
            let commit_output = Command::new("git")
                .args(["commit", "-m", &full_msg])
                .output()?;

            if commit_output.status.success() {
                Ok(String::from_utf8_lossy(&commit_output.stdout).to_string())
            } else {
                Ok(format!("Commit failed: {}", 
                    String::from_utf8_lossy(&commit_output.stderr)))
            }
        } else {
            // For non-commit operations, create contextual command
            let status_output = Command::new("git")
                .arg("status")
                .output()?;
            
            let prompt = format!(
                r#"Convert this Git request into an appropriate command: '{}'
Current status:
{}

Requirements:
1. Use proper git syntax
2. Consider the repository state
3. Follow git best practices

Response format:
DANGEROUS: true/false
COMMAND: <command>"#,
                query,
                String::from_utf8_lossy(&status_output.stdout)
            );

            let response = crate::ai::get_command_suggestion(&prompt, config).await?;
            Ok(response.0)
        }
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
        // Basic command generation without AI
        let base_cmd = match query.to_lowercase() {
            q if q.contains("status") => "git status",
            q if q.contains("diff") => "git diff",
            q if q.contains("log") => "git log",
            q if q.contains("branch") => "git branch",
            _ => "git",
        };
        Ok(base_cmd.to_string())
    }

    fn validate_command(&self, command: &str) -> Result<bool> {
        let dangerous_patterns = [
            "git push --force",
            "git push -f",
            "git clean -f",
            "git reset --hard",
        ];
        Ok(!dangerous_patterns.iter().any(|&pattern| command.contains(pattern)))
    }

    fn explain_command(&self, command: &str) -> Result<String> {
        let prompt = format!(
            r#"Explain this git command:
Command: {}

Consider:
- What changes will occur
- Impact on repo
- Safety implications
- Best practices"#,
            command
        );
        Ok(prompt)
    }
}