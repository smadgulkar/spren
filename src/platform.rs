use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub enum PlatformCommand {
    CreateDir(PathBuf),
    ChangeDir(PathBuf),
    ExecuteCommand { program: String, args: Vec<String> },
}

impl PlatformCommand {
    pub fn from_shell_command(cmd: &str) -> Option<Self> {
        let cmd = cmd.trim();

        if cmd.starts_with("mkdir -p ") || cmd.starts_with("mkdir ") {
            let path = cmd
                .trim_start_matches("mkdir -p ")
                .trim_start_matches("mkdir ")
                .trim()
                .trim_matches('"')
                .replace('/', std::path::MAIN_SEPARATOR_STR);
            Some(Self::CreateDir(PathBuf::from(path)))
        } else if cmd.starts_with("cd ") {
            let path = cmd
                .trim_start_matches("cd ")
                .trim()
                .trim_matches('"')
                .replace('/', std::path::MAIN_SEPARATOR_STR);
            Some(Self::ChangeDir(PathBuf::from(path)))
        } else if cmd.starts_with("npx ") || cmd.starts_with("npm ") {
            // Handle npm/npx commands directly
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            Some(Self::ExecuteCommand {
                program: parts[0].to_string(),
                args: parts[1..].iter().map(|s| s.to_string()).collect(),
            })
        } else {
            // For other commands, use the system shell
            Some(Self::ExecuteCommand {
                program: if cfg!(windows) {
                    "cmd".to_string()
                } else {
                    "sh".to_string()
                },
                args: if cfg!(windows) {
                    vec!["/C".to_string(), cmd.trim_matches('"').to_string()]
                } else {
                    vec!["-c".to_string(), cmd.to_string()]
                },
            })
        }
    }

    pub fn execute(&self) -> Result<String> {
        match self {
            Self::CreateDir(path) => {
                fs::create_dir_all(path).map_err(|e| {
                    anyhow!("Failed to create directory '{}': {}", path.display(), e)
                })?;
                Ok(format!("Created directory: {}", path.display()))
            }
            Self::ChangeDir(path) => {
                std::env::set_current_dir(path).map_err(|e| {
                    anyhow!("Failed to change directory to '{}': {}", path.display(), e)
                })?;
                Ok(format!("Changed directory to: {}", path.display()))
            }
            Self::ExecuteCommand { program, args } => {
                let output = Command::new(program)
                    .args(args)
                    .output()
                    .map_err(|e| anyhow!("Failed to execute command '{}': {}", program, e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if !output.status.success() {
                    return Err(anyhow!(
                        "Command failed: {} {}\nError: {}",
                        program,
                        args.join(" "),
                        stderr
                    ));
                }

                Ok(if stdout.is_empty() { stderr } else { stdout })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_directory() {
        let temp = tempdir().unwrap();
        let test_dir = temp.path().join("test/nested/dir");
        let cmd = PlatformCommand::CreateDir(test_dir.clone());
        cmd.execute().unwrap();
        assert!(test_dir.exists());
    }

    #[test]
    fn test_change_directory() {
        let temp = tempdir().unwrap();
        let cmd = PlatformCommand::ChangeDir(temp.path().to_owned());
        cmd.execute().unwrap();
        assert_eq!(std::env::current_dir().unwrap(), temp.path());
    }

    #[test]
    fn test_command_parsing() {
        if cfg!(windows) {
            assert!(matches!(
                PlatformCommand::from_shell_command(r#"mkdir -p "C:\Users\test""#),
                Some(PlatformCommand::CreateDir(_))
            ));
        } else {
            assert!(matches!(
                PlatformCommand::from_shell_command(r#"mkdir -p "/home/test""#),
                Some(PlatformCommand::CreateDir(_))
            ));
        }
    }
}
