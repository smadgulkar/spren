pub mod chain;

use anyhow::Result;
use std::env;
use std::process::Command;

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute_command(command: &str) -> Result<CommandOutput> {
    let shell_type = crate::shell::ShellType::detect();
    let (shell, args) = shell_type.get_shell_command();

    // Handle cd commands specially
    if command.trim().starts_with("cd ") {
        let path = command.trim()[3..].trim();
        env::set_current_dir(path)?;
        return Ok(CommandOutput {
            stdout: format!("Changed directory to {}", path),
            stderr: String::new(),
        });
    }

    let formatted_command = shell_type.format_command(command);

    let mut cmd = Command::new(shell);
    cmd.args(args).arg(&formatted_command);

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
    let _success = match shell_type {
        crate::shell::ShellType::Bash => output.status.success() && stderr.is_empty(),
        _ => output.status.success(),
    };

    Ok(CommandOutput {
        stdout: stdout.trim().to_string(),
        stderr: stderr.trim().to_string(),
    })
}
