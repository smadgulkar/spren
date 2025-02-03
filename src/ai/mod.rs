use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use tracing::error;

mod error;
mod path_utils;
mod response;
mod schema;
mod tests;

use crate::config::{AIProvider, Config};
pub use error::AIError;
pub use response::{CommandChain, CommandStep};
use response::{ResourceImpact, VersionedResponse};

const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY: u64 = 1000; // milliseconds

// Add these struct definitions at the top level
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(default)]
    text: String,
    #[serde(rename = "type")]
    content_type: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

const SYSTEM_PROMPT: &str = r#"You are a technical project analyzer. Analyze the provided project information and provide detailed insights in the following format:

1. Project Overview
   - Project type and main technologies
   - Architecture and design patterns
   - Key components and their relationships

2. Code Quality Analysis
   - Code organization and structure
   - Potential issues or concerns
   - Best practices followed/missing

3. Dependencies and Infrastructure
   - Key dependencies and their versions
   - Build and deployment setup
   - Infrastructure requirements

4. Recommendations
   - Immediate improvements
   - Long-term suggestions
   - Security considerations

Keep responses technical, specific, and actionable. Focus on patterns and practices rather than individual code lines.

For Windows paths:
- Use single backslashes: C:\Users\name
- Quote paths with spaces: "C:\Program Files\app"
- Do not escape backslashes
- Use mkdir -p for creating directories
- Example: mkdir -p "C:\Users\name\project"
"#;

async fn get_ai_response(query: &str, config: &Config) -> Result<String, AIError> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_response(query, config).await,
        AIProvider::OpenAI => get_openai_response(query, config).await,
    }
}

fn parse_response(response: &str) -> Result<(Vec<CommandStep>, String), AIError> {
    // Try to find JSON in the response if there's additional text
    let json_start = response.find('{').unwrap_or(0);
    let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
    let json_str = &response[json_start..json_end];

    let versioned: VersionedResponse = serde_json::from_str(json_str).map_err(|e| {
        AIError::ParseError(format!(
            "Could not understand AI response - {}. Raw response: {}",
            e, response
        ))
    })?;

    versioned.validate()?;

    Ok((
        versioned
            .response
            .steps
            .into_iter()
            .map(CommandStep::from_schema)
            .collect::<Result<Vec<_>, _>>()?,
        versioned.response.explanation,
    ))
}

pub async fn get_command_chain(query: &str, config: &Config) -> Result<CommandChain, AIError> {
    let response = get_ai_response(query, config).await?;
    let trimmed = response.trim();

    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return Err(AIError::ResponseParseError(
            "AI response was not valid JSON. Please try again.".to_string(),
        ));
    }

    let (steps, explanation) = parse_response(&response)?;
    let total_impact = ResourceImpact::calculate_total(&steps);

    Ok(CommandChain::new(
        steps,
        total_impact,
        explanation,
        response,
    ))
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

    let system_prompt =
        "You are a command-line assistant. You MUST respond with ONLY JSON, no other text.";

    let formatted_query = format!(
        r#"Generate commands for this request. Respond with ONLY this JSON structure:
{{
    "version": "1.0",
    "steps": [
        {{
            "command": "cd \"C:/path\"",
            "explanation": "Brief explanation",
            "is_dangerous": false,
            "estimated_impact": {{
                "cpu_percentage": 0,
                "memory_mb": 0,
                "disk_mb": 0,
                "network_mb": 0,
                "duration_seconds": 1
            }},
            "rollback_command": null
        }}
    ],
    "explanation": "Brief plan"
}}

Request: {}"#,
        query
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&json!({
            "model": config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [{
                "role": "user",
                "content": formatted_query
            }],
            "system": system_prompt,
            "response_format": { "type": "json_object" }
        }))
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            let resp: AnthropicResponse = response
                .json()
                .await
                .map_err(|e| AIError::ParseError(e.to_string()))?;
            if let Some(content) = resp.content.first() {
                Ok(content.text.clone())
            } else {
                Err(AIError::ParseError(
                    "Empty response from Anthropic".to_string(),
                ))
            }
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(AIError::RateLimitError("Rate limit exceeded".to_string()))
        }
        status => {
            let error_text = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {}", error_text);
            Err(AIError::APIError(format!(
                "API request failed with status {}: {}",
                status, error_text
            )))
        }
    }
}

async fn get_openai_response(query: &str, config: &Config) -> Result<String, AIError> {
    let client = reqwest::Client::new();
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| AIError::AuthenticationError("OpenAI API key not found".to_string()))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| AIError::AuthenticationError(e.to_string()))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&json!({
            "model": config.ai.model,
            "messages": [{
                "role": "user",
                "content": query
            }],
            "max_tokens": config.ai.max_tokens
        }))
        .send()
        .await
        .map_err(|e| AIError::NetworkError(e.to_string()))?;

    match response.status() {
        StatusCode::OK => {
            let resp: OpenAIResponse = response
                .json()
                .await
                .map_err(|e| AIError::ParseError(e.to_string()))?;
            if let Some(choice) = resp.choices.first() {
                Ok(choice.message.content.clone())
            } else {
                Err(AIError::ParseError(
                    "Empty response from OpenAI".to_string(),
                ))
            }
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(AIError::RateLimitError("Rate limit exceeded".to_string()))
        }
        status => {
            let error_text = response.text().await.unwrap_or_default();
            error!("OpenAI API error: {}", error_text);
            Err(AIError::APIError(format!(
                "API request failed with status {}: {}",
                status, error_text
            )))
        }
    }
}

pub async fn get_analysis(prompt: &str, config: &Config) -> Result<String, AIError> {
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

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&json!({
            "model": config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "system": SYSTEM_PROMPT
        }))
        .send()
        .await
        .map_err(|e| AIError::NetworkError(e.to_string()))?;

    match response.status() {
        StatusCode::OK => {
            let resp: AnthropicResponse = response
                .json()
                .await
                .map_err(|e| AIError::ParseError(e.to_string()))?;
            if let Some(content) = resp.content.first() {
                Ok(content.text.clone())
            } else {
                Err(AIError::ParseError(
                    "Empty response from Anthropic".to_string(),
                ))
            }
        }
        StatusCode::TOO_MANY_REQUESTS => {
            Err(AIError::RateLimitError("Rate limit exceeded".to_string()))
        }
        status => {
            let error_text = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {}", error_text);
            Err(AIError::APIError(format!(
                "API request failed with status {}: {}",
                status, error_text
            )))
        }
    }
}
