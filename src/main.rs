use anyhow::Result;
use colored::*;
use std::io::{self, Write};
mod ai;
mod config;
mod executor;
mod shell;
mod tools;

use tools::{DevTool, DockerTool, GitTool, KubernetesTool, ToolsConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Load or create config
    let config_path = config::get_config_path()?;
    if !config_path.exists() {
        config::Config::create_default(&config_path)?;
        println!("Created default config file at {:?}", config_path);
        println!("Please update the API key in the config file and restart.");
        return Ok(());
    }

    let config = config::Config::load(&config_path)?;
    let shell_type = shell::ShellType::detect();

    // Initialize tools
    let tools_config = ToolsConfig::detect()?;
    let docker = DockerTool::new();
    let kubernetes = KubernetesTool::new();
    let git = GitTool::new();

    println!("{}", "Spren - Your AI Shell Assistant".green().bold());
    println!("Shell Type: {}", format!("{:?}", shell_type).blue());

    // Display available tools
    println!("\nAvailable Tools:");
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

    println!("\nType 'exit' to quit\n");

    loop {
        print!("spren> ");
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query == "exit" {
            break;
        }

        match process_query(query, &config, &tools_config).await {
            Ok(_) => continue,
            Err(e) => eprintln!("{}: {}", "Error".red().bold(), e),
        }
    }

    Ok(())
}

async fn process_query(query: &str, config: &config::Config, tools: &ToolsConfig) -> Result<()> {
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

    // Show and execute the command
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

    // Execute and handle output
    match executor::execute_command(&command).await {
        Ok(output) => {
            if !output.stderr.is_empty() {
                let suggestion =
                    ai::get_error_suggestion(&command, &output.stdout, &output.stderr, config)
                        .await?;
                println!("\n{}", "Suggestion:".yellow().bold());
                println!("{}", suggestion);
            } else {
                println!("{}", output.stdout);
            }
        }
        Err(e) => {
            println!("\n{}: {}", "Error".red().bold(), e);
        }
    }

    Ok(())
}
