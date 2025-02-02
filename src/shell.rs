// src/shell.rs
use std::env;
use which::which;

#[derive(Debug, Clone)]
pub enum ShellType {
    Powershell,
    Cmd,
    Bash,
    Other(String),
}

impl ShellType {
    pub fn detect() -> Self {
        // First check if we're explicitly in CMD
        if let Ok(comspec) = env::var("ComSpec") {
            if comspec.to_lowercase().contains("cmd.exe") {
                return ShellType::Cmd;
            }
        }

        // Check parent process name
        if let Ok(parent) = env::var("SHELL") {
            let parent = parent.to_lowercase();
            if parent.contains("bash") || parent.contains("zsh") {
                return ShellType::Bash;
            }
        }

        // On Windows, check if powershell is available
        #[cfg(windows)]
        {
            if let Ok(_) = env::var("PSModulePath") {
                return ShellType::Powershell;
            }
            // Additional Windows-specific check
            if which("powershell.exe").is_ok() {
                return ShellType::Powershell;
            }
        }

        // Default to CMD on Windows if nothing else matches
        #[cfg(windows)]
        {
            if which("cmd.exe").is_ok() {
                return ShellType::Cmd;
            }
        }

        ShellType::Other("unknown".to_string())
    }

    pub fn get_shell_name(&self) -> &str {
        match self {
            ShellType::Powershell => "PowerShell",
            ShellType::Cmd => "CMD",
            ShellType::Bash => "Bash",
            ShellType::Other(name) => name,
        }
    }

    pub fn get_shell_command(&self) -> (&str, &[&str]) {
        match self {
            ShellType::Bash => ("sh", &["-c"]),
            ShellType::Powershell => ("powershell", &["-NoProfile", "-NonInteractive", "-Command"]),
            ShellType::Cmd => ("cmd", &["/C"]),
            ShellType::Other(_) => ("sh", &["-c"]), // Default to sh for unknown shells
        }
    }

    pub fn format_command(&self, command: &str) -> String {
        match self {
            ShellType::Cmd => {
                let command = command.trim();
                
                // Handle cd commands
                if command.to_lowercase().starts_with("cd ") {
                    let path = command[3..].trim();
                    let clean_path = path.trim_matches('"')
                        .replace("/", "\\")
                        .replace("\\\\", "\\");
                    return format!("cd /d \"{}\"", clean_path);
                }
                
                // Handle other commands
                command.replace("/", "\\")
                      .replace("\\\\", "\\")
            },
            _ => command.to_string(),
        }
    }
}