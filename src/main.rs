//src/main.rs
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

mod ai;
mod config;
mod executor;
mod shell;
mod tools;

use tools::{CodeGeneratorTool, DevTool, DockerTool, GitTool, KubernetesTool, ToolsConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Load or create config
    let config_path = config::get_config_path()?;
    if !config_path.exists() {
        config::Config::create_default(&config_path)?;
        println!("{}", "Created default config file at:".green());
        println!("{:?}", config_path);
        println!(
            "\n{}",
            "Please update the API key in the config file and restart.".yellow()
        );
        return Ok(());
    }

    let config = config::Config::load(&config_path)?;
    let shell_type = shell::ShellType::detect();

    // Initialize tools
    let tools_config = ToolsConfig::detect()?;
    let docker = DockerTool::new();
    let kubernetes = KubernetesTool::new();
    let git = GitTool::new();

    // Display welcome message and system info
    print_welcome_message(&shell_type, &docker, &kubernetes, &git)?;

    // Main interaction loop
    loop {
        print!("{} ", config.display.prompt_symbol.cyan());
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query.is_empty() {
            continue;
        }

        if query == "exit" || query == "quit" {
            println!("{}", "Goodbye!".green());
            break;
        }

        if query == "help" {
            print_help_message();
            continue;
        }

        match process_query(query, &config, &tools_config).await {
            Ok(_) => continue,
            Err(e) => {
                eprintln!("{}: {}", "Error".red().bold(), e);
                if let Some(suggestion) = get_error_recovery_suggestion(&e).await {
                    println!("\n{}", "Suggestion:".yellow().bold());
                    println!("{}", suggestion);
                }
            }
        }
    }

    Ok(())
}

async fn process_query(query: &str, config: &config::Config, tools: &ToolsConfig) -> Result<()> {
    let start_time = std::time::Instant::now();

    // First, try to get a command chain
    let mut command_chain = ai::get_command_chain(query, config).await?;

    // If we got a chain with multiple steps, execute it
    if command_chain.steps.len() > 1 {
        println!("\n{}", "Executing command chain:".blue().bold());
        println!(
            "{} {} steps detected",
            "►".blue(),
            command_chain.steps.len()
        );

        let _results = executor::execute_command_chain(&mut command_chain).await?;

        // Print execution time if enabled
        if config.display.show_execution_time {
            let duration = start_time.elapsed();
            println!("\n{} {:.2?}", "Execution time:".blue(), duration);
        }

        return Ok(());
    }

    // If it's a single command, process it through the tools system
    let response = tools.process_query(query, config).await?;

    // Parse the response for dangerous commands
    let (command, is_dangerous) = if response.contains("DANGEROUS: true") {
        let parts: Vec<&str> = response.split('\n').collect();
        let command = parts
            .iter()
            .find(|l| l.starts_with("COMMAND: "))
            .map(|l| l.strip_prefix("COMMAND: ").unwrap())
            .unwrap_or(&response);
        (command.to_string(), true)
    } else {
        (response, false)
    };

    // Show command preview if enabled
    if config.display.show_command_preview {
        println!("\n{}", "Suggested command:".blue().bold());
        println!(
            "{}{}",
            command,
            if is_dangerous {
                " [DANGEROUS]".red().bold()
            } else {
                "".into()
            }
        );

        if is_dangerous && config.security.require_confirmation {
            print!("\n{} Continue? [y/N]: ", "WARNING:".yellow().bold());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Operation cancelled by user");
                return Ok(());
            }
        }
    }

    // Execute and handle output
    match executor::execute_command(&command).await {
        Ok(output) => {
            // Don't show error analysis for successful git operations or when stdout is present
            if !output.stderr.is_empty() && (!command.starts_with("git") || !output.success) {
                let suggestion =
                    ai::get_error_suggestion(&command, &output.stdout, &output.stderr, config)
                        .await?;
                println!("\n{}", "Error analysis:".yellow().bold());
                println!("{}", suggestion);
            } else if !output.stdout.trim().is_empty() {
                if config.display.color_output {
                    // Apply some basic syntax highlighting for common output types
                    print_colored_output(&output.stdout);
                } else {
                    println!("{}", output.stdout);
                }
            }

            if config.display.show_execution_time {
                let duration = start_time.elapsed();
                println!("\n{} {:.2?}", "Execution time:".blue(), duration);
            }
        }
        Err(e) => {
            println!("\n{}: {}", "Error".red().bold(), e);

            // Get AI suggestion for the error
            if let Ok(suggestion) =
                ai::get_error_suggestion(&command, "", &e.to_string(), config).await
            {
                println!("\n{}", "Suggestion:".yellow().bold());
                println!("{}", suggestion);
            }
        }
    }

    Ok(())
}

fn print_welcome_message(
    shell_type: &shell::ShellType,
    docker: &DockerTool,
    kubernetes: &KubernetesTool,
    git: &GitTool,
) -> Result<()> {
    println!("{}", "Spren - Your AI Shell Assistant".green().bold());
    println!("Shell Type: {}", format!("{:?}", shell_type).blue());

    println!("\n{}", "Available Tools:".bold());

    if docker.is_available() {
        println!("Docker: {}", "✓".green());
        if let Ok(version) = docker.version() {
            println!("  Version: {}", version);
        }
    }

    if kubernetes.is_available() {
        println!("Kubernetes: {}", "✓".green());
        if let Ok(version) = kubernetes.version() {
            println!("  Version: {}", version);
        }
    }

    if git.is_available() {
        println!("Git: {}", "✓".green());
        if let Ok(version) = git.version() {
            println!("  Version: {}", version);
        }
    }

    // Always show Code Generator as it's always available
    println!("Code Generator: {}", "✓".green());
    println!("  Version: {}", CodeGeneratorTool::new().version()?);

    println!("\nType 'help' for usage information or 'exit' to quit\n");
    Ok(())
}

fn print_help_message() {
    println!("\n{}", "Spren Help".green().bold());
    println!("Available commands:");
    println!("  help    - Show this help message");
    println!("  exit    - Exit the program");
    println!("  quit    - Same as exit");
    println!("\nYou can:");
    println!("- Ask for command suggestions in natural language");
    println!("- Request multi-step operations");
    println!("- Get help with errors and troubleshooting");
    println!("- Work with Git, Docker, and Kubernetes");
    println!("- Generate code files using natural language");
    println!("\nExamples:");
    println!("- \"create a new git branch and switch to it\"");
    println!("- \"show me docker containers using too much memory\"");
    println!("- \"create a deployment in kubernetes\"");
    println!("- \"create an HTML file with a login form\"");
    println!("- \"generate a React component for a user profile\"");
    println!("");
}

fn print_colored_output(output: &str) {
    // Add basic syntax highlighting for common output formats
    for line in output.lines() {
        if line.starts_with('+') || line.starts_with('-') {
            println!("{}", line.green());
        } else if line.contains("error") || line.contains("Error") {
            println!("{}", line.red());
        } else if line.contains("warning") || line.contains("Warning") {
            println!("{}", line.yellow());
        } else if line.starts_with('#') || line.starts_with("//") {
            println!("{}", line.blue());
        } else {
            println!("{}", line);
        }
    }
}

async fn get_error_recovery_suggestion(error: &anyhow::Error) -> Option<String> {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("permission denied") {
        Some("Try running the command with elevated privileges (sudo)".to_string())
    } else if error_str.contains("not found") {
        Some("Check if the required tool is installed and in your PATH".to_string())
    } else if error_str.contains("connection refused") {
        Some("Check if the required service is running".to_string())
    } else if error_str.contains("invalid argument") {
        Some("The command syntax might be incorrect. Check the documentation or try the help command".to_string())
    } else if error_str.contains("no such file") {
        Some(
            "The specified file or directory doesn't exist. Check the path and try again"
                .to_string(),
        )
    } else {
        None
    }
}