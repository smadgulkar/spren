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
    }

    let (shell, args) = shell_type.get_shell_command();
    let formatted_command = shell_type.format_command(command);
    
    let mut cmd = Command::new(shell);
    cmd.args(args).arg(&formatted_command);

    let output = cmd.output()?;
    Ok(output.into())
}

async fn execute_git_command(command: &str) -> Result<CommandOutput> {
    // Split the command into parts
    let parts: Vec<&str> = command.split_whitespace().collect();
    let output = Command::new("git")
        .args(&parts[1..])
        .output()?;

    // For successful git commands, combine stdout and stderr in a meaningful way
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = if stdout.trim().is_empty() {
            stderr.into_owned()
        } else {
            stdout.into_owned()
        };
        
        Ok(CommandOutput {
            stdout: combined,
            stderr: String::new(),
            success: true,
            variables: None,
        })
    } else {
        Ok(output.into())
    }
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

        // Handle output appropriately based on success/failure
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
                // For git commands that succeeded, just show the output
                if command.starts_with("git") {
                    println!("{}", result.stdout.trim());
                } else {
                    println!("\n{}", result.stdout.trim());
                }
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