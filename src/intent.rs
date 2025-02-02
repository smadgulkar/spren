use anyhow::Result;
use crate::code::Language;

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
}

#[derive(Debug, PartialEq)]
pub enum GitOp {
    Status,
    Branch,
    Commit,
    Analyze,
}

pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub async fn analyze(query: &str) -> Result<Intent> {
        let query = query.to_lowercase();
        
        if query.contains("git") || query.contains("commit") || query.contains("branch") {
            let operation = if query.contains("status") || query.contains("changes") {
                GitOp::Status
            } else if query.contains("branch") {
                GitOp::Branch
            } else if query.contains("commit") {
                GitOp::Commit
            } else {
                GitOp::Analyze
            };

            Ok(Intent::GitOperation(GitIntent {
                operation,
                args: query.split_whitespace().map(String::from).collect(),
            }))
        } else if query.contains("generate") || query.contains("create file") || query.contains("new file") {
            let language = if query.contains("rust") {
                Language::Rust
            } else if query.contains("python") {
                Language::Python
            } else if query.contains("typescript") || query.contains("tsx") {
                Language::TypeScript
            } else if query.contains("javascript") || query.contains("jsx") {
                Language::JavaScript
            } else {
                Language::Unknown
            };

            Ok(Intent::CodeGeneration(CodeGenIntent {
                language,
                description: query,
                path: None,
            }))
        } else {
            Ok(Intent::CommandChain)
        }
    }
} 