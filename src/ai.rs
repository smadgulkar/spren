use crate::config::{Config, AIProvider};
use crate::shell::ShellType;
use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceImpact {
    cpu_usage: f32,
    memory_usage: f32,
    disk_usage: f32,
    network_usage: f32,
    estimated_duration: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandStep {
    command: String,
    explanation: String,
    is_dangerous: bool,
    impact: ResourceImpact,
    rollback_command: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandChain {
    steps: Vec<CommandStep>,
    total_impact: ResourceImpact,
    explanation: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AIResponse {
    steps: Vec<CommandStepResponse>,
    explanation: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandStepResponse {
    command: String,
    explanation: String,
    is_dangerous: bool,
    estimated_impact: ResourceImpactResponse,
    rollback_command: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceImpactResponse {
    cpu_percentage: f32,
    memory_mb: f32,
    disk_mb: f32,
    network_mb: f32,
    duration_seconds: f32,
}

impl From<ResourceImpactResponse> for ResourceImpact {
    fn from(response: ResourceImpactResponse) -> Self {
        ResourceImpact {
            cpu_usage: response.cpu_percentage,
            memory_usage: response.memory_mb,
            disk_usage: response.disk_mb,
            network_usage: response.network_mb,
            estimated_duration: Duration::from_secs_f32(response.duration_seconds),
        }
    }
}

pub async fn get_command_chain(query: &str, config: &Config) -> Result<CommandChain> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_command_chain(query, config).await,
        AIProvider::OpenAI => get_openai_command_chain(query, config).await,
    }
}

async fn get_anthropic_command_chain(query: &str, config: &Config) -> Result<CommandChain> {
    let api_key = config.ai.anthropic_api_key.as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

    let prompt = format!(
        "Break down this task into executable {} commands: '{}'\n\
         For each step provide:\n\
         1. The exact command to execute\n\
         2. A brief explanation of what the command does\n\
         3. Safety analysis (is it dangerous?)\n\
         4. Estimated resource impact (CPU%, Memory MB, Disk MB, Network MB, Duration)\n\
         5. A rollback command if applicable\n\
         \n\
         Respond in this JSON format:\n\
         {{\n\
           \"steps\": [\n\
             {{\n\
               \"command\": \"command string\",\n\
               \"explanation\": \"what this step does\",\n\
               \"is_dangerous\": boolean,\n\
               \"estimated_impact\": {{\n\
                 \"cpu_percentage\": float,\n\
                 \"memory_mb\": float,\n\
                 \"disk_mb\": float,\n\
                 \"network_mb\": float,\n\
                 \"duration_seconds\": float\n\
               }},\n\
               \"rollback_command\": \"command to undo this step (or null)\"\n\
             }}\n\
           ],\n\
           \"explanation\": \"overall explanation of the approach\"\n\
         }}",
        shell_name, query
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&serde_json::json!({
            "model": &config.ai.model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Provide detailed step-by-step commands.",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    let ai_response: AIResponse = serde_json::from_str(&response.content[0].text)?;
    
    // Convert AIResponse to CommandChain
    let steps: Vec<CommandStep> = ai_response.steps.into_iter()
        .map(|step| CommandStep {
            command: step.command,
            explanation: step.explanation,
            is_dangerous: step.is_dangerous,
            impact: step.estimated_impact.into(),
            rollback_command: step.rollback_command,
        })
        .collect();

    // Calculate total impact
    let total_impact = ResourceImpact {
        cpu_usage: steps.iter().map(|s| s.impact.cpu_usage).sum(),
        memory_usage: steps.iter().map(|s| s.impact.memory_usage).sum(),
        disk_usage: steps.iter().map(|s| s.impact.disk_usage).sum(),
        network_usage: steps.iter().map(|s| s.impact.network_usage).sum(),
        estimated_duration: Duration::from_secs_f32(
            steps.iter()
                .map(|s| s.impact.estimated_duration.as_secs_f32())
                .sum()
        ),
    };

    Ok(CommandChain {
        steps,
        total_impact,
        explanation: ai_response.explanation,
    })
}

async fn get_openai_command_chain(query: &str, config: &Config) -> Result<CommandChain> {
    let api_key = config.ai.openai_api_key.as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

    let prompt = format!(
        "Break down this task into executable {} commands: '{}'\n\
         For each step provide:\n\
         1. The exact command to execute\n\
         2. A brief explanation of what the command does\n\
         3. Safety analysis (is it dangerous?)\n\
         4. Estimated resource impact (CPU%, Memory MB, Disk MB, Network MB, Duration)\n\
         5. A rollback command if applicable\n\
         \n\
         Respond in this JSON format:\n\
         {{\n\
           \"steps\": [\n\
             {{\n\
               \"command\": \"command string\",\n\
               \"explanation\": \"what this step does\",\n\
               \"is_dangerous\": boolean,\n\
               \"estimated_impact\": {{\n\
                 \"cpu_percentage\": float,\n\
                 \"memory_mb\": float,\n\
                 \"disk_mb\": float,\n\
                 \"network_mb\": float,\n\
                 \"duration_seconds\": float\n\
               }},\n\
               \"rollback_command\": \"command to undo this step (or null)\"\n\
             }}\n\
           ],\n\
           \"explanation\": \"overall explanation of the approach\"\n\
         }}",
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
                    "content": "You are Spren, a helpful command-line assistant. Provide detailed step-by-step commands."
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

    let ai_response: AIResponse = serde_json::from_str(&response.choices[0].message.content)?;
    
    // Convert AIResponse to CommandChain (same as Anthropic implementation)
    let steps: Vec<CommandStep> = ai_response.steps.into_iter()
        .map(|step| CommandStep {
            command: step.command,
            explanation: step.explanation,
            is_dangerous: step.is_dangerous,
            impact: step.estimated_impact.into(),
            rollback_command: step.rollback_command,
        })
        .collect();

    let total_impact = ResourceImpact {
        cpu_usage: steps.iter().map(|s| s.impact.cpu_usage).sum(),
        memory_usage: steps.iter().map(|s| s.impact.memory_usage).sum(),
        disk_usage: steps.iter().map(|s| s.impact.disk_usage).sum(),
        network_usage: steps.iter().map(|s| s.impact.network_usage).sum(),
        estimated_duration: Duration::from_secs_f32(
            steps.iter()
                .map(|s| s.impact.estimated_duration.as_secs_f32())
                .sum()
        ),
    };

    Ok(CommandChain {
        steps,
        total_impact,
        explanation: ai_response.explanation,
    })
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

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

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

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

    let prompt = format!(
        "Break down this task into executable steps: '{}'. \
         For each step provide:\n\
         1. The exact command\n\
         2. A brief explanation\n\
         3. Safety analysis\n\
         4. Estimated resource impact\n\
         Respond in JSON format.",
        query
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

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

    let prompt = format!(
        "Break down this task into executable steps: '{}'. \
         For each step provide:\n\
         1. The exact command\n\
         2. A brief explanation\n\
         3. Safety analysis\n\
         4. Estimated resource impact\n\
         Respond in JSON format.",
        query
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

    let _shell_type = ShellType::detect();
    let shell_name = _shell_type.get_shell_name();

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