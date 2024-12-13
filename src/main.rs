use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::time::Instant;

mod ai;
mod config;
mod executor;
mod shell;

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

    println!("{}", "Spren - Your AI Shell Assistant".green().bold());
    println!("Shell Type: {}", format!("{:?}", shell_type).blue());
    println!("Type 'exit' to quit\n");

    loop {
        print!("spren> ");
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query == "exit" {
            break;
        }

        match process_query(query, &config).await {
            Ok(_) => continue,
            Err(e) => eprintln!("{}: {}", "Error".red().bold(), e),
        }
    }

    Ok(())
}

async fn process_query(query: &str, config: &config::Config) -> Result<()> {
    // Get command suggestion from AI
    let (command, is_dangerous) = ai::get_command_suggestion(query, &config).await?;

    println!("\n{}", "Suggested command:".blue().bold());
    if is_dangerous {
        println!("{} {}", command, "[DANGEROUS]".red().bold());
        println!("\n{}", "This command has been identified as potentially dangerous.".yellow());
        if !config.security.require_confirmation {
            return Ok(());
        }
    } else {
        println!("{}", command);
    }

    if config.security.require_confirmation {
        print!("\nExecute? [y/N] ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if response.trim().to_lowercase() != "y" {
            return Ok(());
        }
    }

    let start_time = Instant::now();
    match executor::execute_command(&command).await {
        Ok(output) => {
            if config.display.show_execution_time {
                println!("\nExecution time: {:?}", start_time.elapsed());
            }

            if !output.stdout.is_empty() {
                println!("\n{}", output.stdout);
            }

            if !output.stderr.is_empty() {
                if output.success {
                    // Command succeeded but had stderr output
                    println!("{}: {}", "Note".yellow().bold(), output.stderr);
                } else {
                    // Command failed
                    println!("{}: {}", "Error".red().bold(), output.stderr);

                    // Get error analysis and suggestion
                    if let Ok(suggestion) = ai::get_error_suggestion(
                        &command,
                        &output.stdout,
                        &output.stderr,
                        &config
                    ).await {
                        println!("\n{}", "Suggestion:".yellow().bold());
                        println!("{}", suggestion);
                    }
                }
            }
        }
        Err(e) => {
            println!("\n{}: {}", "System Error".red().bold(), e);
        }
    }

    Ok(())
}
#[derive(Debug, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

async fn check_for_updates() -> Result<Option<String>> {
    let current_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::new();

    let releases: Vec<GithubRelease> = client
        .get("https://api.github.com/repos/yourusername/spren/releases")
        .header("User-Agent", "spren")
        .send()
        .await?
        .json()
        .await?;

    if let Some(latest) = releases.first() {
        let latest_version = latest.tag_name.trim_start_matches('v');
        if latest_version != current_version {
            return Ok(Some(format!(
                "Update available: {} -> {} ({})",
                current_version, latest_version, latest.html_url
            )));
        }
    }

    Ok(None)
}