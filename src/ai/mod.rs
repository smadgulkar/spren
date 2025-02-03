use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

mod error;
mod response;
mod schema;
mod tests;

pub use self::error::AIError;
pub use self::response::{CommandChain, CommandStep, ResourceImpact};
use crate::config::{AIProvider, Config};
use crate::shell::ShellType;
use response::VersionedResponse;

const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY: u64 = 1000; // milliseconds
const MAX_RETRY_DELAY: u64 = 10000; // 10 seconds max delay

#[derive(Debug)]
struct RetryConfig {
    max_retries: u32,
    initial_delay: u64,
    max_delay: u64,
    retryable_errors: Vec<RetryableError>,
}

#[derive(Debug, PartialEq)]
enum RetryableError {
    RateLimit,
    Network,
    ParseError,
    EmptyResponse,
}

impl RetryConfig {
    fn new() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            initial_delay: INITIAL_RETRY_DELAY,
            max_delay: MAX_RETRY_DELAY,
            retryable_errors: vec![
                RetryableError::RateLimit,
                RetryableError::Network,
                RetryableError::ParseError,
                RetryableError::EmptyResponse,
            ],
        }
    }

    fn should_retry(&self, error: &AIError) -> bool {
        match error {
            AIError::RateLimitError(_) => {
                self.retryable_errors.contains(&RetryableError::RateLimit)
            }
            AIError::NetworkError(_) => self.retryable_errors.contains(&RetryableError::Network),
            AIError::ParseError(_) => self.retryable_errors.contains(&RetryableError::ParseError),
            _ => false,
        }
    }

    fn get_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay * 2u64.pow(attempt);
        Duration::from_millis(delay.min(self.max_delay))
    }
}

async fn with_retries<T, F, Fut>(config: &RetryConfig, f: F) -> Result<T, AIError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, AIError>>,
{
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < config.max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if config.should_retry(&e) {
                    let delay = config.get_delay(attempt);
                    println!("Request failed: {}. Retrying in {:?}...", e, delay);
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                    last_error = Some(e);
                    continue;
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AIError::NetworkError("Max retries exceeded".to_string())))
}

pub async fn get_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let retry_config = RetryConfig::new();

    with_retries(&retry_config, || async {
        match config.ai.provider {
            AIProvider::Anthropic => {
                let response = get_anthropic_response(query, config).await?;
                
                // Try to parse as a versioned response first
                match serde_json::from_str::<VersionedResponse>(&response) {
                    Ok(versioned) => return versioned.into_command_chain(),
                    Err(_) => {
                        // If that fails, try to parse as a git operation
                        match serde_json::from_str::<GitOperationResponse>(&response) {
                            Ok(git_op) => {
                                let command = match git_op.details.operation.as_str() {
                                    "branch" => {
                                        let args = git_op.details.args.join(" ");
                                        if args.is_empty() {
                                            "git branch".to_string()
                                        } else {
                                            format!("git branch {}", args)
                                        }
                                    },
                                    "status" => "git status".to_string(),
                                    _ => return Err(AIError::ParseError(format!(
                                        "Unknown git operation: {}", 
                                        git_op.details.operation
                                    ))),
                                };

                                let description = git_op.details.description.clone();
                                let step = CommandStep {
                                    command,
                                    explanation: description.clone(),
                                    is_dangerous: false,
                                    impact: ResourceImpact {
                                        cpu_usage: 0.1,
                                        memory_usage: 1.0,
                                        disk_usage: 0.0,
                                        network_usage: 0.0,
                                        estimated_duration: Duration::from_secs_f32(0.1),
                                    },
                                    rollback_command: None,
                                };

                                return Ok(CommandChain {
                                    steps: vec![step.clone()],
                                    total_impact: ResourceImpact::calculate_total(&[step]),
                                    explanation: format!("Git operation: {}", description),
                                });
                            },
                            Err(_) => {
                                return Err(AIError::ParseError(
                                    "Failed to parse response as either command chain or git operation".to_string()
                                ));
                            }
                        }
                    }
                }
            }
            AIProvider::OpenAI => get_openai_command_chain(query, config).await,
        }
    })
    .await
}

async fn get_anthropic_response(query: &str, config: &Config) -> Result<String, AIError> {
    let client = reqwest::Client::new();
    let api_key =
        config.ai.anthropic_api_key.as_ref().ok_or_else(|| {
            AIError::AuthenticationError("Anthropic API key not found".to_string())
        })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| AIError::AuthenticationError(e.to_string()))?,
    );
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    #[derive(Deserialize)]
    struct AnthropicResponse {
        content: Vec<AnthropicContent>,
    }

    #[derive(Deserialize)]
    struct AnthropicContent {
        text: String,
    }

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&json!({
            "model": config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [{
                "role": "user",
                "content": query
            }],
            "system": "You are a command-line assistant. Always respond with valid JSON that includes an 'intent' field.",
        }))
        .send()
        .await
        .map_err(|e| AIError::NetworkError(e.to_string()))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| AIError::NetworkError(e.to_string()))?;

    if !status.is_success() {
        return Err(AIError::APIError(format!(
            "API error ({}): {}",
            status, text
        )));
    }

    // Parse the Anthropic response
    let anthropic_response: AnthropicResponse = serde_json::from_str(&text)
        .map_err(|e| AIError::ParseError(format!("Failed to parse Anthropic response: {}", e)))?;

    // Extract the JSON content from the response text
    let content = anthropic_response
        .content
        .first()
        .ok_or_else(|| AIError::ParseError("Empty response from Anthropic".to_string()))?
        .text
        .trim();

    // If the content is wrapped in ```json, extract just the JSON
    let json = if content.starts_with("```json") && content.ends_with("```") {
        content[7..content.len() - 3].trim()
    } else {
        content
    };

    Ok(json.to_string())
}

fn format_prompt(query: &str) -> String {
    format!(
        r#"Analyze the request and return a JSON response in one of these formats:

For git operations:
{{
    "intent": "GitOperation",
    "details": {{
        "operation": "branch",
        "args": [],
        "description": "List all git branches"
    }}
}}

For command chains:
{{
    "intent": "CommandChain",
    "details": {{
        "version": "1.0",
        "steps": [
            {{
                "command": "string",
                "explanation": "string",
                "is_dangerous": boolean,
                "estimated_impact": {{
                    "cpu_percentage": number,
                    "memory_mb": number,
                    "disk_mb": number,
                    "network_mb": number,
                    "duration_seconds": number
                }},
                "rollback_command": null
            }}
        ],
        "explanation": "string"
    }}
}}

For code generation:
{{
    "intent": "CodeGeneration",
    "details": {{
        "language": "rust|python|javascript",
        "description": "Description of code to generate"
    }}
}}

User request: {}"#,
        query
    )
}

pub fn extract_json(text: &str) -> Result<String, AIError> {
    // First try to find complete JSON object
    if let Some(start) = text.find('{') {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, c) in text[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' => escape_next = true,
                '"' => {
                    if !escape_next {
                        in_string = !in_string;
                    }
                }
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        let json = &text[start..=start + i];
                        // Validate it's proper JSON
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json) {
                            if parsed.get("version").is_some() && parsed.get("steps").is_some() {
                                return Ok(json.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // If we couldn't find valid JSON, return the full error for debugging
    Err(AIError::ParseError(format!(
        "Could not find valid JSON response. Response text: {}",
        text.chars().take(200).collect::<String>()
    )))
}

async fn get_openai_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| AIError::ValidationError("OpenAI API key not configured".to_string()))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| AIError::ValidationError(format!("Invalid API key: {}", e)))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let _shell_type = ShellType::detect();
    let _shell_name = _shell_type.get_shell_name();

    let api_url = config
        .ai
        .api_url
        .as_deref()
        .unwrap_or("https://api.openai.com/v1/chat/completions");

    let response = client
        .post(api_url)
        .headers(headers)
        .json(&json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Spren, a helpful command-line assistant. Provide detailed step-by-step commands."
                },
                {
                    "role": "user",
                    "content": format_prompt(query)
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| AIError::NetworkError(e.to_string()))?;

    match response.status() {
        StatusCode::OK => (),
        StatusCode::TOO_MANY_REQUESTS => {
            return Err(AIError::RateLimitError("Rate limit exceeded".to_string()));
        }
        StatusCode::UNAUTHORIZED => {
            return Err(AIError::AuthenticationError("Invalid API key".to_string()));
        }
        status => {
            return Err(AIError::APIError(format!(
                "Unexpected status code: {}",
                status
            )));
        }
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| AIError::NetworkError(format!("Failed to read response body: {}", e)))?;

    let versioned_response: VersionedResponse = serde_json::from_str(&response_text)
        .map_err(|e| AIError::ParseError(format!("Failed to parse API response: {}", e)))?;

    versioned_response.into_command_chain()
}

pub async fn get_ai_response(prompt: &str, config: &Config) -> Result<String, AIError> {
    match config.ai.provider {
        AIProvider::Anthropic => {
            let raw_response = get_anthropic_response(&prompt.to_string(), config).await?;
            // Return the raw JSON string for the caller to parse
            Ok(raw_response)
        }
        AIProvider::OpenAI => {
            let command_chain = get_openai_command_chain(&prompt.to_string(), config).await?;
            let json_value = serde_json::json!({
                "version": "1.0",
                "steps": command_chain.steps,
                "explanation": command_chain.explanation,
                "total_impact": {
                    "cpu_usage": command_chain.total_impact.cpu_usage,
                    "memory_usage": command_chain.total_impact.memory_usage,
                    "disk_usage": command_chain.total_impact.disk_usage,
                    "network_usage": command_chain.total_impact.network_usage,
                    "duration_seconds": command_chain.total_impact.estimated_duration.as_secs_f32(),
                }
            });
            Ok(serde_json::to_string(&json_value)?)
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitOperationResponse {
    intent: String,
    details: GitOperationDetails,
}

#[derive(Debug, Deserialize)]
struct GitOperationDetails {
    operation: String,
    args: Vec<String>,
    description: String,
}

fn convert_git_op_to_command_chain(git_op: GitOperationResponse) -> Result<CommandChain, AIError> {
    let command = match git_op.details.operation.as_str() {
        "branch" => {
            let args = git_op.details.args.join(" ");
            if args.is_empty() {
                "git branch".to_string()
            } else {
                format!("git branch {}", args)
            }
        },
        "status" => "git status".to_string(),
        _ => {
            return Err(AIError::ParseError(format!(
                "Unknown git operation: {}",
                git_op.details.operation
            )))
        }
    };

    let description = git_op.details.description.clone();
    let step = CommandStep {
        command,
        explanation: description.clone(),
        is_dangerous: false,
        impact: ResourceImpact {
            cpu_usage: 0.1,
            memory_usage: 1.0,
            disk_usage: 0.0,
            network_usage: 0.0,
            estimated_duration: Duration::from_secs_f32(0.1),
        },
        rollback_command: None,
    };

    Ok(CommandChain {
        steps: vec![step.clone()],
        total_impact: ResourceImpact::calculate_total(&[step]),
        explanation: format!("Git operation: {}", description),
    })
}

// Re-export theme for use in response.rs
// pub(crate) use crate::theme;
