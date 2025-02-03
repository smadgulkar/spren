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

    pub fn get_status(&self) -> Result<GitChanges> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get git status"));
        }

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

        if output.status.success() {
            let branches = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim_start_matches('*').trim().to_string())
                .collect();
            Ok(branches)
        } else {
            Err(anyhow::anyhow!("Failed to list branches"))
        }
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
}

#[derive(Default, Debug)]
pub struct GitChanges {
    pub staged_modified: Vec<String>,
    pub staged_added: Vec<String>,
    pub staged_deleted: Vec<String>,
    pub unstaged_modified: Vec<String>,
    pub untracked: Vec<String>,
} 