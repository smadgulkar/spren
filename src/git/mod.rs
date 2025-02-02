use anyhow::Result;
use std::process::Command;

pub struct GitManager {
    repo_path: std::path::PathBuf,
}

impl GitManager {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();
        if !repo_path.join(".git").exists() {
            return Err(anyhow::anyhow!("Not a git repository: {:?}", repo_path));
        }
        Ok(Self { repo_path })
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get current branch"))
        }
    }

    pub fn analyze_changes(&self) -> Result<GitChanges> {
        // Get staged and unstaged changes
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()?;

        if !status.status.success() {
            return Err(anyhow::anyhow!("Failed to get git status"));
        }

        let status_output = String::from_utf8_lossy(&status.stdout);
        let mut changes = GitChanges::default();

        for line in status_output.lines() {
            if line.len() < 3 { continue; }
            let (index, working_tree) = line.split_at(2);
            let file = working_tree.trim();
            
            match (index.chars().next(), index.chars().nth(1)) {
                (Some('M'), _) => changes.staged_modified.push(file.to_string()),
                (Some('A'), _) => changes.staged_added.push(file.to_string()),
                (Some('D'), _) => changes.staged_deleted.push(file.to_string()),
                (_, Some('M')) => changes.unstaged_modified.push(file.to_string()),
                (_, Some('?')) => changes.untracked.push(file.to_string()),
                _ => {}
            }
        }

        Ok(changes)
    }
}

#[derive(Default, Debug)]
pub struct GitChanges {
    pub staged_modified: Vec<String>,
    pub staged_added: Vec<String>,
    pub staged_deleted: Vec<String>,
    pub unstaged_modified: Vec<String>,
    pub untracked: Vec<String>,
} 