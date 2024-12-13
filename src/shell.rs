// src/shell.rs
use std::env;

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

    pub fn get_shell_name(&self) -> &str {
        match self {
            ShellType::Bash => "Bash",
            ShellType::PowerShell => "PowerShell",
            ShellType::Cmd => "Command Prompt",
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