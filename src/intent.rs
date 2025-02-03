use anyhow::Result;
use crate::code::Language;
use crate::config::Config;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Intent {
    CommandChain,
    CodeGeneration(CodeGenIntent),
    GitOperation(GitIntent),
    Unknown,
}

#[derive(Debug, PartialEq)]
pub struct CodeGenIntent {
    pub language: Language,
    pub description: String,
    pub path: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct GitIntent {
    pub operation: GitOp,
    pub args: Vec<String>,
    pub description: String,
}

#[derive(Debug, PartialEq)]
pub enum GitOp {
    Status,
    Branch,
    Commit,
    Analyze,
    ListBranches,
    CreateBranch,
    SwitchBranch,
    ShowDiff,
}

pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub async fn analyze(query: &str, config: &Config) -> Result<Intent> {
        // Use AI to determine intent and extract details
        let prompt = format!(
            "Analyze this command and return a JSON response classifying the intent and details:\n\
             Query: {}\n\
             \n\
             Return one of these intents:\n\
             1. CommandChain - for shell commands\n\
             2. GitOperation - for git related operations\n\
             3. CodeGeneration - for creating/modifying code\n\
             \n\
             Example response format:\n\
             ```json\n\
             {{\n\
                \"intent\": \"GitOperation\",\n\
                \"details\": {{\n\
                    \"operation\": \"branch\",\n\
                    \"args\": [\"feature/auth\"],\n\
                    \"description\": \"Create new feature branch for authentication\"\n\
                }}\n\
             }}\n\
             ```",
            query
        );

        let response = crate::ai::get_ai_response(&prompt, config).await?;
        
        // Parse AI response and convert to Intent enum
        let parsed: serde_json::Value = serde_json::from_str(&response)?;
        
        match parsed["intent"].as_str() {
            Some("GitOperation") => {
                let details = &parsed["details"];
                let operation = match details["operation"].as_str() {
                    Some("status") => GitOp::Status,
                    Some("branch") => GitOp::Branch,
                    Some("commit") => GitOp::Commit,
                    Some("list_branches") => GitOp::ListBranches,
                    Some("create_branch") => GitOp::CreateBranch,
                    Some("switch_branch") => GitOp::SwitchBranch,
                    Some("show_diff") => GitOp::ShowDiff,
                    _ => GitOp::Analyze,
                };

                Ok(Intent::GitOperation(GitIntent {
                    operation,
                    args: details["args"].as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect())
                        .unwrap_or_default(),
                    description: details["description"].as_str()
                        .unwrap_or("").to_string(),
                }))
            },
            Some("CodeGeneration") => {
                let details = &parsed["details"];
                Ok(Intent::CodeGeneration(CodeGenIntent {
                    language: Language::from_str(
                        details["language"].as_str().unwrap_or("unknown")
                    ).unwrap_or(Language::Unknown),
                    description: details["description"].as_str()
                        .unwrap_or("").to_string(),
                    path: details["path"].as_str()
                        .map(String::from),
                }))
            },
            _ => Ok(Intent::CommandChain),
        }
    }
} 