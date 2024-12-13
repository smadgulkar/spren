use crate::config::{Config, AIProvider};
use crate::shell::ShellType;
use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
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

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    text: String,
}

pub async fn get_command_suggestion(query: &str, config: &Config) -> Result<(String, bool)> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_command(query, config).await,
        AIProvider::OpenAI => get_openai_command(query, config).await,
    }
}

pub async fn get_error_suggestion(command: &str, stdout: &str, stderr: &str, config: &Config) -> Result<String> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_error(command, stdout, stderr, config).await,
        AIProvider::OpenAI => get_openai_error(command, stdout, stderr, config).await,
    }
}

async fn get_anthropic_error(command: &str, stdout: &str, stderr: &str, config: &Config) -> Result<String> {
    let api_key = config.ai.anthropic_api_key.as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "Analyze this {} command result:\nCommand: {}\nStdout: {}\nStderr: {}\n\
         Explain what happened and suggest improvements. Be specific and brief.",
        shell_name, command, stdout, stderr
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&serde_json::json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Provide clear and concise explanations.",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    Ok(response.content[0].text.trim().to_string())
}

async fn get_anthropic_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config.ai.anthropic_api_key.as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "Convert this natural language query into a {} command: '{}'. \
         Also analyze if this command could be dangerous (e.g., system-wide deletions, \
         format operations, etc). You must respond in exactly this format:\nDANGEROUS: true/false\nCOMMAND: <command>",
        shell_name, query
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&serde_json::json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Respond only in the specified format.",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    parse_ai_response(&response.content[0].text)
}

async fn get_openai_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config.ai.openai_api_key.as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "Convert this natural language query into a {} command: '{}'. \
         Also analyze if this command could be dangerous (e.g., system-wide deletions, \
         format operations, etc). You must respond in exactly this format:\nDANGEROUS: true/false\nCOMMAND: <command>",
        shell_name, query
    );

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&serde_json::json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Spren, a helpful command-line assistant. Respond only in the specified format."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    parse_ai_response(&response.choices[0].message.content)
}

async fn get_openai_error(command: &str, stdout: &str, stderr: &str, config: &Config) -> Result<String> {
    let api_key = config.ai.openai_api_key.as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "Analyze this {} command result:\nCommand: {}\nStdout: {}\nStderr: {}\n\
         Explain what happened and suggest improvements. Be specific and brief.",
        shell_name, command, stdout, stderr
    );

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&serde_json::json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Spren, a helpful command-line assistant. Provide clear and concise explanations."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(response.choices[0].message.content.trim().to_string())
}

fn parse_ai_response(response: &str) -> Result<(String, bool)> {
    let lines: Vec<&str> = response.trim().split('\n').collect();

    let dangerous_line = lines.iter()
        .find(|line| line.to_lowercase().contains("dangerous"))
        .ok_or_else(|| anyhow!("Could not find DANGEROUS line in response"))?;

    let command_line = lines.iter()
        .find(|line| line.to_lowercase().contains("command"))
        .ok_or_else(|| anyhow!("Could not find COMMAND line in response"))?;

    let is_dangerous = dangerous_line.to_lowercase().contains("true");
    let command = command_line
        .replace("COMMAND:", "")
        .replace("Command:", "")
        .trim()
        .to_string();

    Ok((command, is_dangerous))
}