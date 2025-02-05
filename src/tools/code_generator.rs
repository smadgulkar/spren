//src/tools/code_generator.rs
use super::DevTool;
use super::HealthStatus;
use super::HealthStatusType;
use crate::config;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

pub struct CodeGeneratorTool;

impl CodeGeneratorTool {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, query: &str, config: &config::Config) -> Result<String> {
        if !self.is_safe_query(query) {
            return Err(anyhow!("Query contains unsafe operations"));
        }

        let filename = if let Some(name) = self.extract_explicit_filename(query) {
            if !self.is_safe_filename(&name) {
                return Err(anyhow!("Unsafe filename detected"));
            }
            name
        } else {
            self.suggest_filename(query, config).await?
        };

        let prompt = format!(
            r#"Write code for '{filename}' based on this request: '{query}'

Requirements:
- Complete, working implementation
- Clear comments and documentation
- Follow language best practices
- Include error handling
- Make the code efficient and well-structured

Format your response exactly as:
```<language>
<code here>
```"#
        );

        let (content, _) = crate::ai::get_command_suggestion(&prompt, config).await?;

        if let Some(code) = self.extract_code_block(&content) {
            if !self.is_safe_content(&code) {
                return Err(anyhow!("Generated code contains unsafe operations"));
            }
            fs::write(Path::new(&filename), code)?;
            Ok(format!("Created file: {}", filename))
        } else {
            Err(anyhow!(
                "No code was generated. Please try rephrasing your request."
            ))
        }
    }

    fn extract_code_block(&self, content: &str) -> Option<String> {
        let mut in_code_block = false;
        let mut code = String::new();
        let mut first_block = true;

        for line in content.lines() {
            if line.starts_with("```") {
                if !in_code_block && first_block {
                    in_code_block = true;
                    first_block = false;
                    continue;
                } else if in_code_block {
                    break;
                }
            } else if in_code_block {
                code.push_str(line);
                code.push('\n');
            }
        }

        let code = code.trim();
        if code.is_empty() {
            None
        } else {
            Some(code.to_string())
        }
    }

    // Rest of the implementation remains unchanged
    fn is_safe_query(&self, query: &str) -> bool {
        let unsafe_patterns = [
            "system", "exec", "shell", "delete", "remove", "format", "disk", "registry", "regedit",
            "/etc/",
        ];
        !unsafe_patterns
            .iter()
            .any(|&pattern| query.to_lowercase().contains(&pattern.to_lowercase()))
    }

    fn is_safe_filename(&self, filename: &str) -> bool {
        let safe_extensions = ["py", "js", "html", "css", "txt", "md", "json"];
        let has_safe_extension = filename.split('.').last().map_or(false, |ext| {
            safe_extensions.contains(&ext.to_lowercase().as_str())
        });

        let safe_chars = filename
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_');

        has_safe_extension && safe_chars && !filename.contains("..")
    }

    fn is_safe_content(&self, content: &str) -> bool {
        let unsafe_patterns = [
            "import os",
            "import subprocess",
            "exec(",
            "eval(",
            "require('child_process')",
            "Process.Start",
        ];
        !unsafe_patterns
            .iter()
            .any(|&pattern| content.contains(pattern))
    }

    fn extract_explicit_filename(&self, query: &str) -> Option<String> {
        let query = query.to_lowercase();
        let patterns = ["called ", "named ", "file "];

        for pattern in patterns {
            if let Some(idx) = query.find(pattern) {
                let after_pattern = &query[idx + pattern.len()..];
                if let Some(end_idx) = after_pattern.find(char::is_whitespace) {
                    let filename = &after_pattern[..end_idx];
                    if filename.contains('.') {
                        return Some(filename.to_string());
                    }
                } else if after_pattern.contains('.') {
                    return Some(after_pattern.to_string());
                }
            }
        }
        None
    }

    async fn suggest_filename(&self, query: &str, config: &config::Config) -> Result<String> {
        let file_type = if query.to_lowercase().contains("python") {
            "py"
        } else if query.to_lowercase().contains("html") {
            "html"
        } else if query.to_lowercase().contains("css") {
            "css"
        } else if query.to_lowercase().contains("javascript") {
            "js"
        } else {
            "py"
        };

        let prompt = format!(
            r#"Suggest a filename for this code: '{}'
Response format:
FILENAME: <name>.{}"#,
            query, file_type
        );

        let (response, _) = crate::ai::get_command_suggestion(&prompt, config).await?;

        for line in response.lines() {
            if line.starts_with("FILENAME:") {
                let filename = line.replace("FILENAME:", "").trim().to_string();
                if filename.contains('.') && self.is_safe_filename(&filename) {
                    return Ok(filename);
                }
            }
        }

        Ok(format!("script.{}", file_type))
    }
}

impl DevTool for CodeGeneratorTool {
    // DevTool trait implementation remains unchanged
    fn name(&self) -> &'static str {
        "code-generator"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn version(&self) -> Result<String> {
        Ok("1.0.0".to_string())
    }

    fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            status: HealthStatusType::Healthy,
            message: "Code generator is ready".to_string(),
            timestamp: chrono::Utc::now(),
        })
    }

    fn generate_command(&self, query: &str) -> Result<String> {
        if !self.is_safe_query(query) {
            return Err(anyhow!("Query contains unsafe operations"));
        }
        Ok(format!("generate {}", query))
    }

    fn validate_command(&self, command: &str) -> Result<bool> {
        let forbidden_paths = vec![
            "/etc/",
            "/usr/",
            "/bin/",
            "/sbin/",
            "C:\\Windows\\",
            "C:\\Program Files\\",
            "System32",
            "%SystemRoot%",
        ];

        Ok(!forbidden_paths.iter().any(|path| command.contains(path)))
    }

    fn explain_command(&self, command: &str) -> Result<String> {
        if !self.is_safe_query(command) {
            return Err(anyhow!("Cannot explain unsafe command"));
        }
        Ok(format!(
            "This command will generate code based on the following request: {}",
            command
        ))
    }
}
