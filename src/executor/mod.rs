pub mod chain;

use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

impl ToString for CommandOutput {
    fn to_string(&self) -> String {
        if self.success() {
            self.stdout.clone()
        } else {
            format!("Error ({}): {}", self.exit_code, self.stderr)
        }
    }
}

pub async fn execute_command(command: &str) -> Result<CommandOutput> {
    let shell_type = crate::shell::ShellType::detect();

    // Validate command before execution
    if let Err(e) = validate_command(&shell_type, command) {
        return Ok(CommandOutput {
            command: command.to_string(),
            stdout: String::new(),
            stderr: e.to_string(),
            exit_code: -1,
        });
    }

    let (shell, args) = shell_type.get_shell_command();
    let formatted_command = shell_type.format_command(command);

    let mut cmd = Command::new(shell);
    cmd.args(args)
        .arg(&formatted_command)
        .current_dir(std::env::current_dir()?); // Explicitly set current directory

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Clean up the output by removing excessive newlines and whitespace
    let stdout = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Note: PowerShell and CMD might write to stderr even on success
    let success = match shell_type {
        crate::shell::ShellType::Bash => output.status.success() && stderr.is_empty(),
        _ => output.status.success(),
    };

    Ok(CommandOutput {
        command: command.to_string(),
        stdout: stdout.trim().to_string(),
        stderr: stderr.trim().to_string(),
        exit_code: if success { 0 } else { -1 },
    })
}

fn validate_command(shell_type: &crate::shell::ShellType, command: &str) -> Result<()> {
    match shell_type {
        crate::shell::ShellType::Cmd => {
            // Check for problematic CMD patterns
            if command.contains("\\\\") {
                return Err(anyhow::anyhow!(
                    "Invalid path format with double backslashes"
                ));
            }

            // Special handling for cd commands
            if command.trim().to_lowercase().starts_with("cd ") {
                let path = command.trim_start_matches("cd ").trim();
                if path.starts_with("\\\\") {
                    return Err(anyhow::anyhow!(
                        "UNC paths are not supported for cd command"
                    ));
                }
            }
        }
        _ => {} // Add validation for other shells as needed
    }
    Ok(())
}
