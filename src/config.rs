use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub ai: AIConfig,
    pub security: SecurityConfig,
    pub display: DisplayConfig,
    pub shell: ShellConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIConfig {
    pub provider: AIProvider,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    Anthropic,
    OpenAI,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub dangerous_commands: HashSet<String>,
    pub require_confirmation: bool,
    pub max_output_size: usize,
    pub allowed_directories: Vec<String>,
    pub disable_dangerous_commands: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub show_execution_time: bool,
    pub color_output: bool,
    pub verbose_mode: bool,
    pub show_command_preview: bool,
    pub prompt_symbol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    pub preferred_shell: Option<String>,
    pub shell_aliases: std::collections::HashMap<String, String>,
    pub environment_variables: std::collections::HashMap<String, String>,
    pub history_size: usize,
    pub enable_auto_correction: bool,
}

impl Config {
    pub fn load(config_path: &PathBuf) -> Result<Self> {
        let config_str = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn create_default(config_path: &PathBuf) -> Result<()> {
        if let Some(dir) = config_path.parent() {
            fs::create_dir_all(dir)?;
        }

        let default_config = Config {
            ai: AIConfig {
                provider: AIProvider::Anthropic,
                anthropic_api_key: Some("your-anthropic-api-key-here".to_string()),
                openai_api_key: Some("your-openai-api-key-here".to_string()),
                model: "".to_string(),
                max_tokens: 150,
                temperature: 0.7,
            },
            security: SecurityConfig {
                dangerous_commands: [
                    // Unix/Linux dangerous commands
                    "rm -rf",
                    "mkfs",
                    "dd",
                    "shutdown",
                    "reboot",
                    "> /dev",
                    "format",

                    // PowerShell dangerous commands
                    "Remove-Item -Recurse",
                    "Format-Volume",
                    "Stop-Computer",
                    "Restart-Computer",
                    "Remove-Item -Force",

                    // CMD dangerous commands
                    "rmdir /s",
                    "format ",
                    "del /f",
                    "shutdown",
                ].iter().map(|&s| s.to_string()).collect(),
                require_confirmation: true,
                max_output_size: 1024 * 1024, // 1MB
                allowed_directories: vec!["~".to_string(), "./".to_string()],
                disable_dangerous_commands: false,
            },
            display: DisplayConfig {
                show_execution_time: true,
                color_output: true,
                verbose_mode: false,
                show_command_preview: true,
                prompt_symbol: "❯".to_string(),
            },
            shell: ShellConfig {
                preferred_shell: None, // Auto-detect by default
                shell_aliases: std::collections::HashMap::new(),
                environment_variables: std::collections::HashMap::new(),
                history_size: 1000,
                enable_auto_correction: true,
            },
        };

        let toml_string = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }

    pub fn update(&self, config_path: &PathBuf) -> Result<()> {
        let toml_string = toml::to_string_pretty(&self)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".config").join("spren").join("config.toml"))
}

// Helper function to merge user config with defaults
pub fn merge_with_defaults(user_config: Config) -> Config {
    // Implementation would merge any missing fields from default config
    // while preserving user-specified values
    user_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        Config::create_default(&config_path)?;
        assert!(config_path.exists());

        let config = Config::load(&config_path)?;
        assert_eq!(config.ai.provider, AIProvider::Anthropic);
        assert!(config.security.require_confirmation);

        Ok(())
    }

    #[test]
    fn test_dangerous_commands() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        Config::create_default(&config_path)?;
        let config = Config::load(&config_path)?;

        assert!(config.security.dangerous_commands.contains("rm -rf"));
        assert!(config.security.dangerous_commands.contains("Format-Volume"));

        Ok(())
    }
}