//src/executor.rs
use crate::ai::CommandChain;
use crate::shell::ShellType;
use anyhow::{anyhow, Result};
use colored::*;
use regex::Regex;
use std::io::Write;
use std::process::{Command, Output};

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub variables: Option<Vec<(String, String)>>,
}

impl From<Output> for CommandOutput {
    fn from(output: Output) -> Self {
        CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
            variables: None,
        }
    }
}

pub async fn execute_command(command: &str) -> Result<CommandOutput> {
    let shell_type = ShellType::detect();
    
    // Special handling for different command types
    if command.starts_with("git") {
        return execute_git_command(command).await;
    } else if command.starts_with("docker") {
        return execute_docker_command(command).await;
    }

    let (shell, args) = shell_type.get_shell_command();
    let formatted_command = shell_type.format_command(command);
    
    let mut cmd = Command::new(shell);
    cmd.args(args).arg(&formatted_command);

    let output = cmd.output()?;
    Ok(output.into())
}

async fn execute_git_command(command: &str) -> Result<CommandOutput> {
    // Handle git commands with intelligence
    if command.contains("commit") {
        return handle_git_commit(command).await;
    }

    // For other git commands, execute directly
    let output = Command::new("git")
        .args(command.trim_start_matches("git ").split_whitespace())
        .output()?;

    Ok(output.into())
}

async fn handle_git_commit(_: &str) -> Result<CommandOutput> {
    // First, check if there are unstaged changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;

    if !status_output.status.success() {
        return Ok(status_output.into());
    }

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.is_empty() {
        return Ok(CommandOutput {
            stdout: "No changes to commit".to_string(),
            stderr: String::new(),
            success: false,
            variables: None,
        });
    }

    // Get detailed diff for changed files
    let diff_output = Command::new("git")
        .args(["diff", "--staged"])
        .output()?;
    let staged_diff = String::from_utf8_lossy(&diff_output.stdout);

    let unstaged_diff_output = Command::new("git")
        .args(["diff"])
        .output()?;
    let unstaged_diff = String::from_utf8_lossy(&unstaged_diff_output.stdout);

    // Analyze changes to generate appropriate commit message
    let commit_message = generate_commit_message(&status, &staged_diff, &unstaged_diff)?;

    // Add files if needed
    if !status.lines().all(|line| line.starts_with(" M") || line.starts_with("M")) {
        println!("{}", "Adding changed files...".blue());
        let add_output = Command::new("git")
            .args(["add", "."])
            .output()?;

        if !add_output.status.success() {
            return Ok(add_output.into());
        }
    }

    // Execute the commit with generated message
    println!("{} {}", "Committing with message:".blue(), commit_message);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()?;

    Ok(output.into())
}

fn generate_commit_message(status: &str, _: &str, _: &str) -> Result<String> {
    let mut changes = Vec::new();
    
    // Analyze file changes
    for line in status.lines() {
        let status_code = &line[0..2];
        let file = line[3..].to_string();
        
        match status_code.trim() {
            "M" => changes.push(format!("Modified {}", file)),
            "A" => changes.push(format!("Added {}", file)),
            "D" => changes.push(format!("Deleted {}", file)),
            "R" => changes.push(format!("Renamed {}", file)),
            _ => changes.push(format!("Changed {}", file)),
        }
    }

    // Check for specific file types and patterns
    let has_rust = status.contains(".rs");
    let has_tests = status.contains("test") || status.contains("spec");
    let has_docs = status.contains(".md") || status.contains("doc");
    let is_dependency_change = status.contains("Cargo.toml") || status.contains("package.json");

    // Generate appropriate prefix
    let prefix = if has_tests {
        "test:"
    } else if has_docs {
        "docs:"
    } else if is_dependency_change {
        "deps:"
    } else if has_rust {
        "feat:"
    } else {
        "chore:"
    };

    // Generate message body
    let mut message = if changes.len() == 1 {
        format!("{} {}", prefix, changes[0])
    } else {
        let summary = if has_rust {
            "Update Rust implementations".to_string()
        } else if has_tests {
            "Update test suite".to_string()
        } else if has_docs {
            "Update documentation".to_string()
        } else if is_dependency_change {
            "Update dependencies".to_string()
        } else {
            "Multiple changes".to_string()
        };

        format!("{} {}", prefix, summary)
    };

    // Add details if there are multiple changes
    if changes.len() > 1 {
        message.push_str("\n\n- ");
        message.push_str(&changes.join("\n- "));
    }

    Ok(message)
}

async fn execute_docker_command(command: &str) -> Result<CommandOutput> {
    let output = Command::new("docker")
        .args(command.trim_start_matches("docker ").split_whitespace())
        .output()?;

    Ok(output.into())
}

pub async fn execute_command_chain(chain: &mut CommandChain) -> Result<Vec<CommandOutput>> {
    let mut results = Vec::new();
    let mut had_error = false;

    for (step_index, step) in chain.steps.iter().enumerate() {
        println!(
            "\n{} Step {} - {}",
            "►".blue(),
            step_index + 1,
            step.description
        );

        if step.dangerous || step.requires_confirmation {
            println!(
                "{} This step is marked as {}",
                "⚠".yellow(),
                if step.dangerous {
                    "dangerous".red()
                } else {
                    "requiring confirmation".yellow()
                }
            );

            print!("Continue with this step? [y/N]: ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                return Err(anyhow!("Step {} was skipped by user", step_index + 1));
            }
        }

        // Handle dependencies
        let command = if let Some(dep) = &step.dependent_on {
            let value = chain
                .context
                .get(dep)
                .ok_or_else(|| anyhow!("Missing required dependency: {}", dep))?;
            interpolate_variables(&step.command, &[(dep.clone(), value.clone())])
        } else {
            step.command.clone()
        };

        println!("{} Executing: {}", "▷".blue(), command);

        let result = execute_command(&command).await?;

        // Extract and store variables if specified
        if let Some(var_name) = &step.provides {
            let value = extract_output_value(&result.stdout)?;
            chain.context.insert(var_name.clone(), value);
        }

        // Handle command output
        if !result.success {
            had_error = true;
            println!("{} Step {} failed", "✗".red(), step_index + 1);

            if !result.stderr.is_empty() {
                println!("\n{}", "Error output:".red());
                println!("{}", result.stderr);
            }

            print!("\nContinue with remaining steps? [y/N]: ");
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                return Ok(results);
            }
        } else {
            println!(
                "{} Step {} completed successfully",
                "✓".green(),
                step_index + 1
            );
            
            if !result.stdout.trim().is_empty() {
                println!("\n{}", result.stdout.trim());
            }
        }

        results.push(result);
    }

    if had_error {
        println!(
            "\n{} Command chain completed with some errors",
            "⚠".yellow()
        );
    } else {
        println!("\n{} Command chain completed successfully", "✓".green());
    }

    Ok(results)
}

fn interpolate_variables(command: &str, variables: &[(String, String)]) -> String {
    let mut result = command.to_string();
    for (name, value) in variables {
        result = result.replace(&format!("${{{}}}", name), value);
    }
    result
}

fn extract_output_value(output: &str) -> Result<String> {
    let trimmed = output.trim();

    if !trimmed.contains('\n') {
        return Ok(trimmed.to_string());
    }

    // Try to find lines that look like they contain values
    let value_patterns = [
        Regex::new(r"(?i)id:\s*(.+)").unwrap(),
        Regex::new(r"([0-9a-f]{7,40})").unwrap(), // Git commit hashes
        Regex::new(r"(?i)name:\s*(.+)").unwrap(),
    ];

    for pattern in &value_patterns {
        if let Some(captures) = pattern.captures(trimmed) {
            if let Some(matched) = captures.get(1) {
                return Ok(matched.as_str().trim().to_string());
            }
        }
    }

    Ok(trimmed
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(trimmed)
        .to_string())
}