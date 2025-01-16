use anyhow::Result;

#[derive(Debug, PartialEq)]
pub enum Intent {
    CommandChain,
    CodeGeneration,
    GitOperation,
    Unknown,
}

pub struct IntentAnalyzer;

impl IntentAnalyzer {
    pub async fn analyze(query: &str) -> Result<Intent> {
        // Simple keyword-based analysis for now
        let query = query.to_lowercase();
        
        if query.contains("git") || query.contains("commit") || query.contains("branch") {
            Ok(Intent::GitOperation)
        } else if query.contains("generate") || query.contains("create file") || query.contains("new file") {
            Ok(Intent::CodeGeneration)
        } else {
            Ok(Intent::CommandChain)
        }
    }
} 