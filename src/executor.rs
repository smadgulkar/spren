// src/executor.rs
use anyhow::Result;
use std::process::Command;
use crate::shell::ShellType;

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub async fn execute_command(command: &str) -> Result<CommandOutput> {
    let shell_type = ShellType::detect();
    let (shell, args) = shell_type.get_shell_command();
    let formatted_command = shell_type.format_command(command);

    let mut cmd = Command::new(shell);
    cmd.args(args).arg(&formatted_command);

    let output = cmd.output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Note: PowerShell and CMD might write to stderr even on success
    let success = match shell_type {
        ShellType::Bash => output.status.success() && stderr.is_empty(),
        _ => output.status.success()
    };

    Ok(CommandOutput {
        stdout,
        stderr,
        success
    })
}