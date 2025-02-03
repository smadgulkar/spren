use crate::config::Config;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitManager {
    repo_path: PathBuf,
    config: Config,
}

#[derive(Debug, serde::Deserialize)]
struct GitIntent {
    operation: String,
    args: Vec<String>,
    explanation: String,
}

#[derive(Default, Debug)]
pub struct GitChanges {
    pub staged_modified: Vec<String>,
    pub staged_added: Vec<String>,
    pub staged_deleted: Vec<String>,
    pub unstaged_modified: Vec<String>,
    pub untracked: Vec<String>,
}

impl GitManager {
    pub fn new(path: impl AsRef<Path>, config: Config) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();
        if !repo_path.join(".git").exists() {
            return Err(anyhow::anyhow!("Not a git repository: {:?}", repo_path));
        }
        Ok(Self { repo_path, config })
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn get_status(&self) -> Result<GitChanges> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()?;

        let mut changes = GitChanges::default();

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.len() < 3 {
                continue;
            }

            let status = &line[0..2];
            let file = line[3..].to_string();

            match status {
                "M " => changes.staged_modified.push(file),
                " M" => changes.unstaged_modified.push(file),
                "A " => changes.staged_added.push(file),
                "D " => changes.staged_deleted.push(file),
                "??" => changes.untracked.push(file),
                _ => (),
            }
        }

        Ok(changes)
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["branch", "--list"])
            .current_dir(&self.repo_path)
            .output()?;

        let branches = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim_start_matches('*').trim().to_string())
            .collect();
        Ok(branches)
    }

    pub fn switch_branch(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["checkout", branch_name])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to switch branch"))
        }
    }

    pub fn create_branch(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to create branch"))
        }
    }

    pub async fn execute(&self, operation: &str) -> Result<String> {
        let prompt = format!(
            r#"Analyze this git-related request and convert it into a git command.
            Request: {}
            Return a JSON response in this format:
            {{
                "operation": "status|commit|branch|checkout|...",
                "args": ["arg1", "arg2"],
                "explanation": "Human readable explanation"
            }}"#,
            operation
        );

        let response = crate::ai::get_ai_response(&prompt, &self.config).await?;
        let intent: GitIntent = serde_json::from_str(&response)?;

        let output = self.execute_git_command(&intent.operation, &intent.args)?;

        let explain_prompt = format!(
            r#"Explain this git command output in clear, concise bullet points. Be direct and skip any intro text:
            Command: git {} {}
            Output: {}
            "#,
            intent.operation,
            intent.args.join(" "),
            output
        );

        let explanation = crate::ai::get_ai_response(&explain_prompt, &self.config).await?;

        // Format the output nicely
        let formatted = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&explanation) {
            if let Some(response) = json.get("response") {
                match response {
                    serde_json::Value::Array(items) => items
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| format!("• {}", s))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    serde_json::Value::String(s) => s.to_string(),
                    _ => explanation.clone(),
                }
            } else {
                explanation.clone()
            }
        } else {
            explanation.clone()
        };

        Ok(formatted)
    }

    pub async fn suggest_commit_message(&self) -> Result<String> {
        let diff = self.get_staged_diff()?;
        let prompt = format!(
            r#"Generate a clear, concise commit message for these changes:
            {}
            Return just the commit message, no JSON wrapper."#,
            diff
        );

        Ok(crate::ai::get_ai_response(&prompt, &self.config).await?)
    }

    pub async fn analyze_merge_conflict(&self) -> Result<String> {
        let conflicts = self.get_conflicts()?;
        let prompt = format!(
            r#"Analyze these merge conflicts and suggest resolutions:
            {}
            Explain in clear terms how to resolve each conflict."#,
            conflicts
        );

        Ok(crate::ai::get_ai_response(&prompt, &self.config).await?)
    }

    fn execute_git_command(&self, operation: &str, args: &[String]) -> Result<String> {
        let mut cmd = Command::new("git");
        cmd.arg(operation).args(args).current_dir(&self.repo_path);

        let output = cmd.output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_staged_diff(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&self.repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_conflicts(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "--diff-filter=U", "--raw"])
            .current_dir(&self.repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
