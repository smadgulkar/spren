use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use reqwest::StatusCode;
use serde_json::json;
use serde::Deserialize;
use std::time::Duration;

mod schema;
mod response;
mod error;
mod tests;

pub use error::AIError;
pub use response::{CommandChain, CommandStep};
use response::VersionedResponse;
use crate::config::{Config, AIProvider};
use crate::shell::ShellType;

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
            AIError::RateLimitError(_) => self.retryable_errors.contains(&RetryableError::RateLimit),
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
            AIProvider::Anthropic => get_anthropic_command_chain(query, config).await,
            AIProvider::OpenAI => get_openai_command_chain(query, config).await,
        }
    }).await
}

async fn get_anthropic_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let api_key = config.ai.anthropic_api_key.as_ref()
        .ok_or_else(|| AIError::ValidationError("Anthropic API key not configured".to_string()))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)
        .map_err(|e| AIError::ValidationError(format!("Invalid API key: {}", e)))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let _shell_type = ShellType::detect();
    let _shell_name = _shell_type.get_shell_name();

    let api_url = config.ai.api_url.as_deref()
        .unwrap_or("https://api.anthropic.com/v1/messages");

    let response = client
        .post(api_url)
        .headers(headers)
        .json(&json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Provide detailed step-by-step commands.",
            "messages": [
                {
                    "role": "user",
                    "content": format_prompt(query, _shell_name)
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
            let error_body = response.text().await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(AIError::APIError(format!(
                "Unexpected status code: {} - Response: {}", 
                status, error_body
            )));
        }
    }

    let response_text = response.text().await
        .map_err(|e| AIError::NetworkError(format!("Failed to read response body: {}", e)))?;

    println!("DEBUG: Raw AI response: {}", response_text);

    #[derive(Debug, Deserialize)]
    struct AnthropicResponse {
        #[serde(default)]
        content: Vec<AnthropicContent>,
        #[serde(default)]
        messages: Vec<AnthropicContent>,
    }

    #[derive(Debug, Deserialize)]
    struct AnthropicContent {
        text: String,
        #[serde(default)]
        content: String,
    }

    let anthropic_response: AnthropicResponse = serde_json::from_str(&response_text)
        .map_err(|e| AIError::ParseError(format!(
            "Failed to parse Anthropic response: {} - Raw response: {}", 
            e, response_text
        )))?;

    let content = if !anthropic_response.content.is_empty() {
        &anthropic_response.content
    } else {
        &anthropic_response.messages
    };

    let ai_response = content.last()
        .ok_or_else(|| AIError::ParseError(format!("Empty response from Anthropic: {}", response_text)))?;

    let response_text = if !ai_response.text.is_empty() {
        &ai_response.text
    } else {
        &ai_response.content
    };

    // Find and validate JSON content
    let json_text = extract_json(response_text)?;

    let versioned_response: VersionedResponse = serde_json::from_str(&json_text)
        .map_err(|e| AIError::ParseError(format!(
            "Failed to parse command chain JSON: {} - Response text: {}", 
            e, json_text
        )))?;

    versioned_response.into_command_chain()
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
                },
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
    let api_key = config.ai.openai_api_key.as_ref()
        .ok_or_else(|| AIError::ValidationError("OpenAI API key not configured".to_string()))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))
        .map_err(|e| AIError::ValidationError(format!("Invalid API key: {}", e)))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let _shell_type = ShellType::detect();
    let _shell_name = _shell_type.get_shell_name();

    let api_url = config.ai.api_url.as_deref()
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
                    "content": format_prompt(query, _shell_name)
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
            return Err(AIError::APIError(format!("Unexpected status code: {}", status)));
        }
    }

    let response_text = response.text().await
        .map_err(|e| AIError::NetworkError(format!("Failed to read response body: {}", e)))?;

    let versioned_response: VersionedResponse = serde_json::from_str(&response_text)
        .map_err(|e| AIError::ParseError(format!("Failed to parse API response: {}", e)))?;

    versioned_response.into_command_chain()
}

fn format_prompt(query: &str, shell_name: &str) -> String {
    let shell_template = if shell_name == "CMD" {
        "For Windows CMD:\n\
         1. Use 'echo.' to write content to files\n\
         2. Use > for redirection\n\
         3. Use ^ to escape special characters\n\
         4. For HTML files, escape special characters and quotes\n\
         5. Use type command for multi-line content\n"
    } else {
        ""
    };

    format!(
        "{}\n\
         Task: {}\n\
         Shell: {}\n\
         \n\
         Return a JSON response with these steps:\n\
         1. Create parent directories if needed\n\
         2. Create and write the file content\n\
         3. Verify the file was created\n\
         \n\
         Use this exact JSON format:\n\
         ```json\n\
         {{\n\
           \"version\": \"1.0\",\n\
           \"steps\": [\n\
             {{\n\
               \"command\": \"mkdir test_2\",\n\
               \"explanation\": \"Create directory if it doesn't exist\",\n\
               \"is_dangerous\": false,\n\
               \"estimated_impact\": {{\n\
                 \"cpu_percentage\": 0.1,\n\
                 \"memory_mb\": 1.0,\n\
                 \"disk_mb\": 0.1,\n\
                 \"network_mb\": 0.0,\n\
                 \"duration_seconds\": 0.1\n\
               }},\n\
               \"rollback_command\": \"rmdir test_2\"\n\
             }},\n\
             {{\n\
               \"command\": \"echo ^<!DOCTYPE html^>^<html^>... > index.html\",\n\
               \"explanation\": \"Create HTML file with content\",\n\
               \"is_dangerous\": false,\n\
               \"estimated_impact\": {{\n\
                 \"cpu_percentage\": 0.1,\n\
                 \"memory_mb\": 1.0,\n\
                 \"disk_mb\": 0.1,\n\
                 \"network_mb\": 0.0,\n\
                 \"duration_seconds\": 0.1\n\
               }},\n\
               \"rollback_command\": \"del index.html\"\n\
             }}\n\
           ],\n\
           \"explanation\": \"Create directory and HTML file with login form\"\n\
         }}\n\
         ```\n\
         Do not include any other text or explanations outside the JSON block.",
        shell_template, query, shell_name
    )
} 