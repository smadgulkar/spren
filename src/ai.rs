//src/ai.rs
use crate::config::{AIProvider, Config};
use crate::shell::ShellType;
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const TIMEOUT_DURATION: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct CommandChain {
    pub steps: Vec<CommandStep>,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CommandStep {
    pub command: String,
    pub description: String,
    pub dangerous: bool,
    pub requires_confirmation: bool,
    pub dependent_on: Option<String>,
    pub provides: Option<String>,
    pub validate_output: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<OpenAIMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    content: String,
}

pub async fn get_command_chain(query: &str, config: &Config) -> Result<CommandChain> {
    let shell_type = ShellType::detect();
    
    // Special handling for git commit operations
    if query.to_lowercase().contains("commit") && query.to_lowercase().contains("git") {
        let mut steps = Vec::new();
        
        // Get current git status
        let status_output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .output()?;
        
        let status = String::from_utf8_lossy(&status_output.stdout);
        
        // Get diff for context
        let diff_output = std::process::Command::new("git")
            .args(["diff"])
            .output()?;
        let diff = String::from_utf8_lossy(&diff_output.stdout);
        
        // Step 1: Add files if there are unstaged changes
        if !status.is_empty() {
            steps.push(CommandStep {
                command: "git add .".to_string(),
                description: "Stage all modified files".to_string(),
                dangerous: false,
                requires_confirmation: false,
                dependent_on: None,
                provides: None,  // Remove dependency tracking for simpler flow
                validate_output: None,
            });
        }
        
        // Generate meaningful commit message based on changes
        let mut message = String::new();
        let files: Vec<_> = status.lines().collect();
        
        if files.iter().any(|f| f.contains(".rs")) {
            message.push_str("feat: Update Rust implementation - ");
            let rs_files: Vec<_> = files.iter()
                .filter(|f| f.contains(".rs"))
                .map(|f| &f[3..])
                .collect();
            message.push_str(&rs_files.join(", "));
        } else if files.iter().any(|f| f.contains("test")) {
            message.push_str("test: Update test suite");
        } else if files.iter().any(|f| f.contains(".md")) {
            message.push_str("docs: Update documentation");
        } else {
            message.push_str("chore: Update project files");
        }
        
        // Add details about changed files
        if !files.is_empty() {
            message.push_str("\n\nChanges:");
            for file in files {
                if file.len() > 3 {
                    message.push_str(&format!("\n- {}", &file[3..]));
                }
            }
        }
        
        // Step 2: Commit with generated message
        steps.push(CommandStep {
            command: format!("git commit -m \"{}\"", message.trim()),
            description: "Commit changes with descriptive message".to_string(),
            dangerous: false,
            requires_confirmation: true,
            dependent_on: None,  // Remove dependency tracking for simpler flow
            provides: None,
            validate_output: None,  // Remove validation for simpler flow
        });
        
        return Ok(CommandChain {
            steps,
            context: HashMap::new(),
        });
    }
    
    let prompt = format!(
        r#"Analyze this query and break it down into a sequence of shell commands: '{}'

Current Environment:
- Shell: {}
- OS Type: {}

Requirements:
1. Break down complex operations into logical steps
2. Ensure proper error checking between steps
3. Handle data dependencies between commands
4. Include validation rules for critical steps
5. Consider security implications

Response Format:
STEPS: <number_of_steps>
BEGIN_STEP: 1
COMMAND: <exact command to execute>
DESCRIPTION: <clear description of what this step does>
DANGEROUS: true/false
REQUIRES_CONFIRMATION: true/false
DEPENDENT_ON: <variable_name or NONE>
PROVIDES: <variable_name or NONE>
VALIDATE_OUTPUT: <validation rule or NONE>
END_STEP"#,
        query,
        shell_type.get_shell_name(),
        if cfg!(windows) { "Windows" } else { "Unix-like" }
    );

    let response = match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_response(&prompt, config).await?,
        AIProvider::OpenAI => get_openai_response(&prompt, config).await?,
    };

    parse_command_chain(&response)
}

pub async fn get_command_suggestion(query: &str, config: &Config) -> Result<(String, bool)> {
    let shell_type = ShellType::detect();
    let is_code_request = query.to_lowercase().contains("create")
        && (query.to_lowercase().contains("file") || query.to_lowercase().contains("script"));

    let prompt = if is_code_request {
        format!(
            r#"Generate code based on this request: '{}'

Your response must be in this exact format:
DANGEROUS: false
COMMAND: write
CODE: ```<language>
<code here>
```"#,
            query
        )
    } else {
        let tool_specific_prompt = if query.contains("docker") {
            "For Docker commands, use proper Docker CLI syntax (docker <command>)"
        } else if query.contains("git") {
            "For Git commands, use proper Git CLI syntax (git <command>)"
        } else if query.contains("kubectl") || query.contains("kubernetes") {
            "For Kubernetes commands, use proper kubectl syntax (kubectl <command>)"
        } else {
            match shell_type {
                ShellType::PowerShell => "Use PowerShell commands (Get-*, Set-*, etc.)",
                ShellType::Cmd => "Use CMD.exe commands with proper syntax",
                ShellType::Bash => "Use Bash commands with proper syntax",
            }
        };

        format!(
            r#"Convert this request into a command: '{}'

Environment:
- Shell: {}
- OS: {}
Instructions:
{}

Your response must be in this exact format:
DANGEROUS: true/false
COMMAND: <command>"#,
            query,
            shell_type.get_shell_name(),
            if cfg!(windows) { "Windows" } else { "Unix-like" },
            tool_specific_prompt
        )
    };

    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_command(&prompt, config).await,
        AIProvider::OpenAI => get_openai_command(&prompt, config).await,
    }
}

async fn get_anthropic_command(prompt: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config
        .ai
        .anthropic_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT_DURATION)
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static(ANTHROPIC_API_VERSION),
    );
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&AnthropicRequest {
            model: config.ai.model.clone(),
            max_tokens: config.ai.max_tokens,
            temperature: config.ai.temperature,
            system: "You are an expert developer and system administrator. Provide responses in the exact format specified.".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        })
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    parse_ai_response(&response.content[0].text)
}


async fn get_openai_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT_DURATION)
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let system_prompt = create_system_prompt(&shell_type, "command");

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&OpenAIRequest {
            model: config.ai.model.clone(),
            max_tokens: config.ai.max_tokens,
            temperature: config.ai.temperature,
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: format!("Convert this request into a shell command: '{}'\nResponse format:\nDANGEROUS: true/false\nCOMMAND: <command>", query),
                },
            ],
        })
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    parse_ai_response(&response.choices[0].message.content)
}

async fn get_anthropic_response(prompt: &str, config: &Config) -> Result<String> {
    let api_key = config
        .ai
        .anthropic_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT_DURATION)
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static(ANTHROPIC_API_VERSION),
    );
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&AnthropicRequest {
            model: config.ai.model.clone(),
            max_tokens: config.ai.max_tokens,
            temperature: config.ai.temperature,
            system: "You are Spren, an expert command-line assistant.".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        })
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    Ok(response.content[0].text.trim().to_string())
}

async fn get_openai_response(prompt: &str, config: &Config) -> Result<String> {
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT_DURATION)
        .build()?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&OpenAIRequest {
            model: config.ai.model.clone(),
            max_tokens: config.ai.max_tokens,
            temperature: config.ai.temperature,
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: "You are Spren, an expert command-line assistant.".to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
        })
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(response.choices[0].message.content.trim().to_string())
}

pub async fn get_error_suggestion(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    let prompt = format!(
        r#"Analyze this command result and provide a helpful solution:
Command: {}
Stdout: {}
Stderr: {}

Provide analysis in this format:
1. Problem Identification
2. Root Cause
3. Solution Steps
4. Prevention Tips"#,
        command, stdout, stderr
    );

    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_response(&prompt, config).await,
        AIProvider::OpenAI => get_openai_response(&prompt, config).await,
    }
}

fn parse_ai_response(response: &str) -> Result<(String, bool)> {
    let lines: Vec<&str> = response.lines().collect();

    let mut dangerous = false;
    let mut command = String::new();
    let mut code_block = String::new();
    let mut in_code_block = false;

    for line in lines {
        let line = line.trim();
        if line.starts_with("DANGEROUS:") {
            dangerous = line.to_lowercase().contains("true");
        } else if line.starts_with("COMMAND:") {
            command = line.replace("COMMAND:", "").trim().to_string();
        } else if line.starts_with("```") {
            in_code_block = !in_code_block;
        } else if in_code_block {
            code_block.push_str(line);
            code_block.push('\n');
        }
    }

    if command.is_empty() {
        return Err(anyhow!("No command found in response"));
    }

    if command == "write" && !code_block.is_empty() {
        Ok((code_block, dangerous))
    } else {
        Ok((command, dangerous))
    }
}
fn parse_command_chain(response: &str) -> Result<CommandChain> {
    let mut steps = Vec::new();
    let mut current_step: Option<CommandStep> = None;
    let context = HashMap::new();

    for line in response.lines() {
        let line = line.trim();

        match line {
            line if line.starts_with("BEGIN_STEP:") => {
                current_step = Some(CommandStep {
                    command: String::new(),
                    description: String::new(),
                    dangerous: false,
                    requires_confirmation: false,
                    dependent_on: None,
                    provides: None,
                    validate_output: None,
                });
            }
            line if line.starts_with("COMMAND:") => {
                if let Some(step) = current_step.as_mut() {
                    step.command = line.replace("COMMAND:", "").trim().to_string();
                }
            }
            line if line.starts_with("DESCRIPTION:") => {
                if let Some(step) = current_step.as_mut() {
                    step.description = line.replace("DESCRIPTION:", "").trim().to_string();
                }
            }
            line if line.starts_with("DANGEROUS:") => {
                if let Some(step) = current_step.as_mut() {
                    step.dangerous = line.to_lowercase().contains("true");
                }
            }
            line if line.starts_with("REQUIRES_CONFIRMATION:") => {
                if let Some(step) = current_step.as_mut() {
                    step.requires_confirmation = line.to_lowercase().contains("true");
                }
            }
            line if line.starts_with("DEPENDENT_ON:") => {
                if let Some(step) = current_step.as_mut() {
                    let replaced = line.replace("DEPENDENT_ON:", "");
                    let value = replaced.trim();
                    step.dependent_on = if value == "NONE" {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
            }
            line if line.starts_with("PROVIDES:") => {
                if let Some(step) = current_step.as_mut() {
                    let replaced = line.replace("PROVIDES:", "");
                    let value = replaced.trim();
                    step.provides = if value == "NONE" {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
            }
            line if line.starts_with("VALIDATE_OUTPUT:") => {
                if let Some(step) = current_step.as_mut() {
                    let replaced = line.replace("VALIDATE_OUTPUT:", "");
                    let value = replaced.trim();
                    step.validate_output = if value == "NONE" {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
            }
            line if line.starts_with("END_STEP") => {
                if let Some(step) = current_step.take() {
                    // Validate the step has required fields before adding
                    if step.command.is_empty() {
                        return Err(anyhow!("Step is missing command"));
                    }
                    if step.description.is_empty() {
                        return Err(anyhow!("Step is missing description"));
                    }
                    steps.push(step);
                }
            }
            _ => {}
        }
    }

    Ok(CommandChain { steps, context })
}

// Helper function to extract structured error information
pub fn parse_error_output(stderr: &str) -> HashMap<String, String> {
    let mut error_info = HashMap::new();

    // Common error patterns
    let patterns = [
        ("permission denied", "permissions"),
        ("command not found", "missing_command"),
        ("No such file or directory", "missing_file"),
        ("Connection refused", "connection"),
        ("Invalid argument", "invalid_args"),
    ];

    for (pattern, key) in patterns.iter() {
        if stderr.to_lowercase().contains(&pattern.to_lowercase()) {
            error_info.insert(key.to_string(), stderr.to_string());
        }
    }

    error_info
}

// Helper function for creating consistent system prompts
fn create_system_prompt(shell_type: &ShellType, task_type: &str) -> String {
    match task_type {
        "command" => format!(
            "You are Spren, an expert command-line assistant specializing in {} commands. \
             Prioritize safety and best practices. Be precise and concise.",
            shell_type.get_shell_name()
        ),
        "error" => format!(
            "You are Spren, an expert in diagnosing {} shell errors. \
             Provide clear, actionable solutions with step-by-step guidance.",
            shell_type.get_shell_name()
        ),
        "chain" => format!(
            "You are Spren, an expert in creating {} shell command sequences. \
             Break down complex tasks into logical, safe steps with proper error handling.",
            shell_type.get_shell_name()
        ),
        _ => "You are Spren, an expert command-line assistant.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_parse_command_chain() {
        let response = r#"STEPS: 2
BEGIN_STEP: 1
COMMAND: echo "Hello"
DESCRIPTION: Print greeting
DANGEROUS: false
REQUIRES_CONFIRMATION: false
DEPENDENT_ON: NONE
PROVIDES: greeting
VALIDATE_OUTPUT: NONE
END_STEP
BEGIN_STEP: 2
COMMAND: echo "${greeting} World"
DESCRIPTION: Complete the greeting
DANGEROUS: false
REQUIRES_CONFIRMATION: false
DEPENDENT_ON: greeting
PROVIDES: NONE
VALIDATE_OUTPUT: NONE
END_STEP"#;

        let chain = parse_command_chain(response).unwrap();
        assert_eq!(chain.steps.len(), 2);
        assert_eq!(chain.steps[0].command, "echo \"Hello\"");
        assert_eq!(chain.steps[1].dependent_on, Some("greeting".to_string()));
    }

    #[test]
    fn test_parse_error_output() {
        let stderr = "Permission denied: /usr/local/bin";
        let error_info = parse_error_output(stderr);
        assert!(error_info.contains_key("permissions"));
    }

    #[test]
    fn test_create_system_prompt() {
        let shell_type = ShellType::Bash;
        let prompt = create_system_prompt(&shell_type, "command");
        assert!(prompt.contains("Bash"));
        assert!(prompt.contains("safety"));
    }
}
