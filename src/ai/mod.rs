use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use reqwest::StatusCode;
use serde_json::json;
use serde::Deserialize;
use std::time::Duration;
use serde_json::Value;

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

pub async fn get_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_command_chain(query, config).await,
        AIProvider::OpenAI => get_openai_command_chain(query, config).await,
    }
}

async fn get_anthropic_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let mut retries = 0;
    let mut last_error = None;

    while retries < MAX_RETRIES {
        match try_anthropic_request(query, config).await {
            Ok(chain) => return Ok(chain),
            Err(e) => {
                match e {
                    AIError::RateLimitError(_) | AIError::NetworkError(_) => {
                        let delay = INITIAL_RETRY_DELAY * 2u64.pow(retries);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        retries += 1;
                        last_error = Some(e);
                        continue;
                    }
                    _ => return Err(e),
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AIError::NetworkError("Max retries exceeded".to_string())))
}

async fn try_anthropic_request(query: &str, config: &Config) -> Result<CommandChain, AIError> {
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

    println!("Raw response: {}", response_text);

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
        .map_err(|e| AIError::ParseError(format!("Failed to parse Anthropic response: {} - Raw response: {}", e, response_text)))?;

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

    println!("Extracted response text: {}", response_text);

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
    let json_start = text.find('{')
        .ok_or_else(|| AIError::ParseError("No JSON found in response".to_string()))?;
    let potential_json = &text[json_start..];

    // Debug: Try direct parsing first
    println!("Attempting direct parse...");
    if let Ok(parsed) = serde_json::from_str::<Value>(potential_json) {
        if parsed.get("version").is_some() && parsed.get("steps").is_some() {
            println!("Direct parse successful!");
            return Ok(potential_json.to_string());
        }
    }

    // Debug: Try finding valid JSON substring
    println!("Attempting to find valid JSON substring...");
    let mut brace_count = 0;
    let mut last_valid_pos = None;
    let mut in_string = false;
    let mut escape_next = false;
    let mut in_heredoc = false;  // Add tracking for heredoc
    let chars: Vec<char> = potential_json.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' if !in_heredoc => in_string = !in_string,
            '@' if !in_string => {
                // Check for heredoc start/end
                if i > 0 && chars[i-1] == '"' {
                    in_heredoc = !in_heredoc;
                }
            },
            '{' if !in_string && !in_heredoc => brace_count += 1,
            '}' if !in_string && !in_heredoc => {
                brace_count -= 1;
                if brace_count == 0 {
                    let test_json = &potential_json[..=i];
                    if let Ok(parsed) = serde_json::from_str::<Value>(test_json) {
                        if parsed.get("version").is_some() && parsed.get("steps").is_some() {
                            last_valid_pos = Some(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(end_pos) = last_valid_pos {
        println!("Found valid JSON substring!");
        let json_str = &potential_json[..=end_pos];
        return Ok(json_str.to_string());
    }

    // If all else fails, try preprocessing the JSON
    println!("Attempting JSON preprocessing...");
    let preprocessed = preprocess_json(potential_json);
    
    if let Ok(parsed) = serde_json::from_str::<Value>(&preprocessed) {
        if parsed.get("version").is_some() && parsed.get("steps").is_some() {
            println!("Preprocessing successful!");
            return Ok(preprocessed);
        }
    }

    Err(AIError::ParseError("Could not parse response as valid JSON".to_string()))
}

fn preprocess_json(json: &str) -> String {
    // First normalize all newlines to \n
    let normalized = json.replace("\r\n", "\n");
    
    let lines: Vec<&str> = normalized.lines().collect();
    let mut processed = String::new();
    let mut in_heredoc = false;
    let mut current_command = String::new();
    let mut heredoc_content = String::new();
    
    for line in lines {
        let trimmed = line.trim();
        
        if trimmed.contains("\"command\"") {
            // Start of a new command
            if !current_command.is_empty() {
                processed.push_str(&current_command);
                processed.push('\n');
            }
            current_command.clear();
            heredoc_content.clear();
            
            if trimmed.contains("@\"") || trimmed.contains("@'") {
                in_heredoc = true;
                // Keep the command part but replace heredoc start
                let cmd_part = trimmed.split("@").next().unwrap_or("");
                current_command.push_str(cmd_part);
                current_command.push('"');
            } else {
                current_command.push_str(trimmed);
            }
        } else if in_heredoc {
            if trimmed.contains("\"@") || trimmed.contains("'@") {
                // End of heredoc - combine everything
                in_heredoc = false;
                
                // Properly escape the heredoc content
                let escaped = heredoc_content
                    .trim_end()
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n");
                
                current_command.push_str(&escaped);
                current_command.push('"');
                current_command.push_str(trimmed.split("@").last().unwrap_or(""));
                
                processed.push_str(&current_command);
                processed.push('\n');
                current_command.clear();
            } else {
                // Accumulate heredoc content
                if !heredoc_content.is_empty() {
                    heredoc_content.push('\n');
                }
                heredoc_content.push_str(line); // Use original line to preserve indentation
            }
        } else {
            // Regular JSON line
            if !current_command.is_empty() {
                processed.push_str(&current_command);
                processed.push('\n');
                current_command.clear();
            }
            processed.push_str(trimmed);
            processed.push('\n');
        }
    }
    
    // Add any remaining content
    if !current_command.is_empty() {
        processed.push_str(&current_command);
        processed.push('\n');
    }

    processed
}

async fn get_openai_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let mut retries = 0;
    let mut last_error = None;

    while retries < MAX_RETRIES {
        match try_openai_request(query, config).await {
            Ok(chain) => return Ok(chain),
            Err(e) => {
                match e {
                    AIError::RateLimitError(_) | AIError::NetworkError(_) => {
                        let delay = INITIAL_RETRY_DELAY * 2u64.pow(retries);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        retries += 1;
                        last_error = Some(e);
                        continue;
                    }
                    _ => return Err(e),
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AIError::NetworkError("Max retries exceeded".to_string())))
}

async fn try_openai_request(query: &str, config: &Config) -> Result<CommandChain, AIError> {
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
    let python_template = if shell_name == "CMD" {
        "When creating Python scripts:\n\
         1. Use a try-except block around input() calls\n\
         2. Add a pause at the end using input('Press Enter to exit...')\n\
         3. Handle EOF and KeyboardInterrupt exceptions\n"
    } else {
        ""
    };

    format!(
        "{}\n\
         Break down this task into executable {} commands: '{}'\n\
         For each step provide:\n\
         1. The exact command to execute\n\
         2. A brief explanation of what the command does\n\
         3. Safety analysis (is it dangerous?)\n\
         4. Estimated resource impact (CPU%, Memory MB, Disk MB, Network MB, Duration)\n\
         5. A rollback command if applicable\n\
         \n\
         Respond in this JSON format with version:\n\
         {{\n\
           \"version\": \"1.0\",\n\
           \"steps\": [\n\
             {{\n\
               \"command\": \"command string\",\n\
               \"explanation\": \"what this step does\",\n\
               \"is_dangerous\": false,\n\
               \"estimated_impact\": {{\n\
                 \"cpu_percentage\": 0.1,\n\
                 \"memory_mb\": 1.0,\n\
                 \"disk_mb\": 0.0,\n\
                 \"network_mb\": 0.0,\n\
                 \"duration_seconds\": 0.1\n\
               }},\n\
               \"rollback_command\": \"command to undo this step (or null)\"\n\
             }}\n\
           ],\n\
           \"explanation\": \"overall explanation of the approach\"\n\
         }}",
        python_template, shell_name, query
    )
} 