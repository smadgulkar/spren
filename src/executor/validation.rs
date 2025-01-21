use crate::ai::CommandChain;
use crate::shell::ShellType;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashSet;

pub struct CommandValidator {
    dangerous_patterns: HashSet<String>,
    shell_type: ShellType,
}

impl CommandValidator {
    pub fn new(shell_type: ShellType) -> Self {
        let mut dangerous_patterns = HashSet::new();
        dangerous_patterns.insert("rm -rf".to_string());
        dangerous_patterns.insert("dd".to_string());
        dangerous_patterns.insert("> /dev/".to_string());
        dangerous_patterns.insert("mkfs".to_string());
        // Add more dangerous patterns

        Self {
            dangerous_patterns,
            shell_type,
        }
    }

    pub fn validate_chain(&self, chain: &CommandChain) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // Validate each step
        for (i, step) in chain.steps.iter().enumerate() {
            // Check for dangerous commands
            if self.is_dangerous_command(&step.command) {
                report.add_warning(ValidationWarning {
                    step_index: i,
                    message: format!("Potentially dangerous command: {}", step.command),
                    severity: WarningSeverity::High,
                });
            }

            // Validate command syntax
            if let Err(e) = self.validate_command_syntax(&step.command) {
                report.add_error(ValidationError {
                    step_index: i,
                    message: format!("Invalid command syntax: {}", e),
                });
            }

            // Check resource impact
            if step.impact.cpu_usage > 0.8 || step.impact.memory_usage > 1000.0 {
                report.add_warning(ValidationWarning {
                    step_index: i,
                    message: "High resource usage detected".to_string(),
                    severity: WarningSeverity::Medium,
                });
            }

            // Validate rollback commands
            if let Some(ref rollback) = step.rollback_command {
                if let Err(e) = self.validate_command_syntax(rollback) {
                    report.add_error(ValidationError {
                        step_index: i,
                        message: format!("Invalid rollback command syntax: {}", e),
                    });
                }
            }
        }

        Ok(report)
    }

    fn is_dangerous_command(&self, command: &str) -> bool {
        self.dangerous_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
    }

    fn validate_command_syntax(&self, command: &str) -> Result<()> {
        match self.shell_type {
            ShellType::Cmd => self.validate_cmd_syntax(command),
            ShellType::PowerShell => self.validate_powershell_syntax(command),
            ShellType::Bash => self.validate_bash_syntax(command),
        }
    }

    fn validate_cmd_syntax(&self, command: &str) -> Result<()> {
        // Basic CMD syntax validation
        if command.contains('|') && !command.contains("||") {
            if !Regex::new(r"\|\s*\w+")?.is_match(command) {
                return Err(anyhow!("Invalid pipe syntax"));
            }
        }
        Ok(())
    }

    fn validate_powershell_syntax(&self, command: &str) -> Result<()> {
        // Basic PowerShell syntax validation
        if command.contains('|') {
            if !Regex::new(r"\|\s*\w+-\w+")?.is_match(command) {
                return Err(anyhow!("Invalid pipeline syntax"));
            }
        }
        Ok(())
    }

    fn validate_bash_syntax(&self, command: &str) -> Result<()> {
        // Basic Bash syntax validation
        if command.contains(';') && !command.contains(";;") {
            if !Regex::new(r";\s*\w+")?.is_match(command) {
                return Err(anyhow!("Invalid command separator"));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_high_severity_warnings(&self) -> bool {
        self.warnings
            .iter()
            .any(|w| matches!(w.severity, WarningSeverity::High))
    }
}

#[derive(Debug)]
pub struct ValidationError {
    pub step_index: usize,
    pub message: String,
}

#[derive(Debug)]
pub struct ValidationWarning {
    pub step_index: usize,
    pub message: String,
    pub severity: WarningSeverity,
}

#[derive(Debug, PartialEq)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
}
