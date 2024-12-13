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

    let formatted_command = match shell_type {
        ShellType::PowerShell => {
            // Wrap PowerShell commands with proper formatting
            format!(
                "$OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8; \
                 $FormatEnumerationLimit = -1; \
                 $result = {}; \
                 if ($result -is [System.Array]) {{ \
                    $result | Format-Table -AutoSize -Wrap | Out-String -Width 120 \
                 }} elseif ($null -ne $result) {{ \
                    $result | Format-Table -AutoSize -Wrap | Out-String -Width 120 \
                 }} else {{ \
                    \"No output\" \
                 }}",
                command
            )
        },
        _ => shell_type.format_command(command)
    };

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
    let success = match shell_type {
        ShellType::Bash => output.status.success() && stderr.is_empty(),
        _ => output.status.success()
    };

    Ok(CommandOutput {
        stdout: stdout.trim().to_string(),
        stderr: stderr.trim().to_string(),
        success
    })
}