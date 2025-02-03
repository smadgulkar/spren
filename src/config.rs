use anyhow::anyhow;
use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const fn default_max_tokens() -> u32 {
    4000
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AIConfig,
    pub security: SecurityConfig,
    pub display: DisplayConfig,
    #[serde(default)]
    pub debug: DebugConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    Anthropic,
    OpenAI,
}

impl Default for AIProvider {
    fn default() -> Self {
        Self::Anthropic
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    #[serde(default)]
    pub provider: AIProvider,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
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
pub struct SecurityConfig {
    pub require_confirmation: bool,
    pub dangerous_commands: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_confirmation: true,
            dangerous_commands: vec!["rm".to_string(), "sudo".to_string()],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayConfig {
    pub show_execution_time: bool,
    pub color_output: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_execution_time: true,
            color_output: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DebugConfig {
    pub show_raw_response: bool,
    pub log_level: String,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_raw_response: false,
            log_level: "info".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AIConfig::default(),
            security: SecurityConfig {
                require_confirmation: true,
                dangerous_commands: vec!["rm".to_string(), "sudo".to_string()],
            },
            display: DisplayConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

impl Config {
    pub fn create_default(path: &PathBuf) -> Result<()> {
        let default_config = Config {
            ai: AIConfig {
                provider: AIProvider::Anthropic,
                anthropic_api_key: None,
                openai_api_key: None,
                model: "claude-3-opus-20240229".to_string(),
                max_tokens: default_max_tokens(),
                api_url: None,
            },
            security: SecurityConfig::default(),
            display: DisplayConfig::default(),
            debug: DebugConfig::default(),
        };

        let toml = toml::to_string_pretty(&default_config)?;
        fs::write(path, toml)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        toml::from_str(&contents).map_err(|e| {
            anyhow!("Failed to parse config file: {}. Make sure 'provider' is either 'anthropic' or 'openai' (lowercase)", e)
        })
    }

    pub fn migrate_config(path: &PathBuf) -> Result<()> {
        let contents = fs::read_to_string(path)?;
        let mut config: toml::Value = toml::from_str(&contents)?;

        // Fix provider case if needed
        if let Some(table) = config.get_mut("ai").and_then(|v| v.as_table_mut()) {
            if let Some(provider) = table.get_mut("provider").and_then(|v| v.as_str()) {
                let fixed_provider = provider.to_lowercase();
                if fixed_provider != provider {
                    table.insert("provider".to_string(), toml::Value::String(fixed_provider));
                    let new_contents = toml::to_string_pretty(&config)?;
                    fs::write(path, new_contents)?;
                }
            }
        }

        Ok(())
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "spren", "spren")
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;

    Ok(proj_dirs.config_dir().join("config.toml"))
}
