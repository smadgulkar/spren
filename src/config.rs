use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AIConfig,
    pub security: SecurityConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: AIProvider,
    pub model: String,
    pub max_tokens: u32,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    #[serde(skip)]
    pub api_url: Option<String>,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::Anthropic,
            model: "claude-3-opus-20240229".to_string(),
            max_tokens: 4000,
            anthropic_api_key: None,
            openai_api_key: None,
            api_url: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AIProvider {
    Anthropic,
    OpenAI,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityConfig {
    pub require_confirmation: bool,
    pub dangerous_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayConfig {
    pub show_execution_time: bool,
    pub color_output: bool,
}

impl Config {
    pub fn create_default(path: &Path) -> Result<()> {
        let config = Config {
            ai: AIConfig {
                provider: AIProvider::Anthropic,
                model: "claude-3-opus-20240229".to_string(),
                max_tokens: 2000,
                anthropic_api_key: None,
                openai_api_key: None,
                api_url: None,
            },
            security: SecurityConfig {
                require_confirmation: true,
                dangerous_commands: vec!["rm".to_string(), "sudo".to_string()],
            },
            display: DisplayConfig {
                show_execution_time: true,
                color_output: true,
            },
        };

        let content = toml::to_string_pretty(&config)?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "spren", "spren")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    
    Ok(proj_dirs.config_dir().join("config.toml"))
}