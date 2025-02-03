use super::{CodeGenParams, CommandChainParams, GitParams, Intent};
use crate::config::Config;
use crate::shell::ShellType;
use anyhow::Result;
use serde_json::Value;

pub struct IntentMatcher;

impl IntentMatcher {
    pub async fn analyze(query: &str, config: &Config) -> Result<Intent> {
        let shell_type = ShellType::detect();
        let prompt = Self::build_prompt(query, &shell_type);
        let response = crate::ai::get_ai_response(&prompt, config).await?;
        let parsed: Value = serde_json::from_str(&response)?;

        match parsed.get("intent").and_then(|v| v.as_str()) {
            Some("GitOperation") => {
                if let Some(details) = parsed.get("details") {
                    let params: GitParams = serde_json::from_value(details.clone())?;
                    Ok(Intent::GitOperation(params))
                } else {
                    Ok(Intent::Unknown)
                }
            }
            Some("CommandChain") => {
                if let Some(details) = parsed.get("details") {
                    let params: CommandChainParams = serde_json::from_value(details.clone())?;
                    Ok(Intent::CommandChain(params))
                } else {
                    Ok(Intent::Unknown)
                }
            }
            Some("CodeGeneration") => {
                if let Some(details) = parsed.get("details") {
                    let params: CodeGenParams = serde_json::from_value(details.clone())?;
                    Ok(Intent::CodeGeneration(params))
                } else {
                    Ok(Intent::Unknown)
                }
            }
            _ => Ok(Intent::Unknown),
        }
    }

    fn build_prompt(query: &str, shell_type: &ShellType) -> String {
        format!(
            r#"Analyze this command and return a JSON response with intent and details.
Query: {query}

Current shell: {}

You are a shell expert. When generating system commands:
1. Consider the current shell type and OS limitations
2. Use commands that don't require admin/root privileges
3. For Windows CMD:
   - Prefer built-in commands over external tools
   - Use "systeminfo" for memory info
   - Use "dir" with appropriate flags for disk info
   - Format commands to be CMD-compatible
4. For PowerShell:
   - Use Get-ComputerInfo for system info
   - Use Get-PSDrive for disk info
   - Format output as tables or lists for readability
5. For Bash:
   - Use "free" for memory info
   - Use "df" for disk info
   - Consider standard flags like -h for human-readable output

Return format:
{{
    "intent": "GitOperation|CommandChain|CodeGeneration",
    "details": {{
        // For CommandChain:
        "commands": ["cmd1", "cmd2"],  // Shell-appropriate commands that work without elevated privileges
        "explanation": "What these commands do and why they were chosen"

        // For GitOperation:
        "operation": "branch|status|commit",
        "args": ["arg1", "arg2"],
        "description": "Operation description"

        // For CodeGeneration:
        "language": "rust|python|javascript",
        "description": "Code description",
        "path": "optional/path/to/file"
    }}
}}

Example for system info in CMD:
{{
    "intent": "CommandChain",
    "details": {{
        "commands": [
            "systeminfo | findstr /C:\"Total Physical Memory\" /C:\"Available Physical Memory\"",
            "dir /s /-c /w"
        ],
        "explanation": "Show system memory and disk usage using built-in CMD commands"
    }}
}}"#,
            shell_type.get_shell_name()
        )
    }
}
