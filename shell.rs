// src/shell.rs
use std::env;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum ShellType {
    Bash,
    PowerShell,
    Cmd,
}

impl ShellType {
    pub fn detect() -> Self {
        if cfg!(windows) {
            // Check if running in PowerShell
            if let Ok(shell_name) = env::var("PSModulePath") {
                if !shell_name.is_empty() {
                    return ShellType::PowerShell;
                }
            }
            // Default to CMD on Windows if not PowerShell
            ShellType::Cmd
        } else {
            // Default to Bash on Unix-like systems
            ShellType::Bash
        }
    }

    pub fn get_shell_command(&self) -> (&str, &[&str]) {
        match self {
            ShellType::Bash => ("sh", &["-c"]),
            ShellType::PowerShell => ("powershell", &["-NoProfile", "-Command"]),
            ShellType::Cmd => ("cmd", &["/C"]),
        }
    }

    pub fn format_command(&self, command: &str) -> String {
        match self {
            ShellType::Bash => command.to_string(),
            ShellType::PowerShell => {
                // Escape single quotes and wrap in single quotes for PowerShell
                format!("'{}'", command.replace('\'', "''"))
            },
            ShellType::Cmd => {
                // Escape special characters for CMD
                command.replace("\"", "\\\"")
            }
        }
    }
}

pub fn is_dangerous_command(command: &str, dangerous_patterns: &[String], shell_type: &ShellType) -> bool {
    let command_lower = command.to_lowercase();

    // Common dangerous patterns across all shells
    let common_dangerous = dangerous_patterns.iter()
        .any(|pattern| command_lower.contains(&pattern.to_lowercase()));

    // Shell-specific dangerous patterns
    let shell_specific_dangerous = match shell_type {
        ShellType::PowerShell => {
            command_lower.contains("remove-item") && command_lower.contains("-recurse") ||
                command_lower.contains("format-volume") ||
                command_lower.contains("stop-computer") ||
                command_lower.contains("restart-computer")
        },
        ShellType::Cmd => {
            command_lower.contains("rmdir /s") ||
                command_lower.contains("format ") ||
                command_lower.contains("shutdown") ||
                command_lower.contains("del /f")
        },
        ShellType::Bash => false  // Already covered by common patterns
    };

    common_dangerous || shell_specific_dangerous
}