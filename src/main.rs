use anyhow::{Result, anyhow};
use colored::*;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use crate::executor::chain::ChainExecutor;
use crate::ai::AIError;

mod ai;
mod config;
mod executor;
mod shell;
mod intent;

use intent::{Intent, IntentAnalyzer};

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
    let intent = IntentAnalyzer::analyze(query).await?;

    match intent {
        Intent::CommandChain => {
            let chain = match ai::get_command_chain(query, &config).await {
                Ok(chain) => chain,
                Err(e) => {
                    match e {
                        AIError::RateLimitError(msg) => {
                            println!("{}: {}. Retrying...", "Rate limit".yellow().bold(), msg);
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            ai::get_command_chain(query, &config).await.map_err(|e| anyhow!(e.to_string()))?
                        }
                        AIError::NetworkError(msg) => {
                            println!("{}: {}. Retrying...", "Network error".yellow().bold(), msg);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            ai::get_command_chain(query, &config).await.map_err(|e| anyhow!(e.to_string()))?
                        }
                        _ => return Err(anyhow!(e.to_string()))
                    }
                }
            };

            let mut executor = ChainExecutor::new(chain);

            // Show preview
            println!("\n{}", "Command Chain Preview:".blue().bold());
            println!("{}", executor.preview());

            if config.security.require_confirmation {
                print!("\nExecute this command chain? [y/N/s(step-by-step)] ");
                io::stdout().flush()?;

                let mut response = String::new();
                io::stdin().read_line(&mut response)?;
                let response = response.trim().to_lowercase();

                match response.as_str() {
                    "y" => {
                        // Execute all steps
                        let start_time = Instant::now();
                        match executor.execute_all().await {
                            Ok(outputs) => {
                                println!("\n{}", "✓ Command chain completed successfully".green());
                                if config.display.show_execution_time {
                                    println!("Total execution time: {:?}", start_time.elapsed());
                                }
                                for output in outputs {
                                    if !output.stdout.is_empty() {
                                        println!("{}", output.stdout);
                                    }
                                    if !output.stderr.is_empty() {
                                        println!("{}: {}", "Note".yellow().bold(), output.stderr);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("\n{}: {}", "Chain execution failed".red().bold(), e);
                                println!("\nWould you like to rollback the changes? [y/N] ");
                                io::stdout().flush()?;
                                
                                let mut rollback = String::new();
                                io::stdin().read_line(&mut rollback)?;
                                if rollback.trim().to_lowercase() == "y" {
                                    match executor.rollback().await {
                                        Ok(_) => println!("✓ Successfully rolled back changes"),
                                        Err(e) => println!("Failed to rollback: {}", e),
                                    }
                                }
                            }
                        }
                    }
                    "s" => {
                        // Step by step execution
                        while !executor.is_complete() {
                            if let Some(step) = executor.current_step_details() {
                                let (current, total) = executor.progress();
                                println!("\n{} ({}/{})", "Current step:".blue().bold(), current + 1, total);
                                println!("Command: {}", step.command);
                                println!("Explanation: {}", step.explanation);
                                
                                print!("\nExecute this step? [y/n/s(skip)] ");
                                io::stdout().flush()?;
                                
                                let mut step_response = String::new();
                                io::stdin().read_line(&mut step_response)?;
                                
                                match step_response.trim().to_lowercase().as_str() {
                                    "y" => {
                                        match executor.execute_next().await {
                                            Ok(Some(output)) => {
                                                if !output.stdout.is_empty() {
                                                    println!("{}", output.stdout);
                                                }
                                                if !output.stderr.is_empty() {
                                                    println!("{}: {}", "Note".yellow().bold(), output.stderr);
                                                }
                                            }
                                            Ok(None) => break,
                                            Err(e) => {
                                                println!("\n{}: {}", "Step failed".red().bold(), e);
                                                return Ok(());
                                            }
                                        }
                                    }
                                    "s" => {
                                        executor.skip_step()?;
                                    }
                                    _ => {
                                        println!("Chain execution cancelled");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        println!("\n{}", "✓ Command chain completed".green());
                    }
                    _ => {
                        println!("Chain execution cancelled");
                        return Ok(());
                    }
                }
            }
        }
        Intent::CodeGeneration | Intent::GitOperation => {
            println!("{}", "This feature is coming soon!".yellow().bold());
        }
        Intent::Unknown => {
            println!("{}", "Could not determine the intent of your query.".red().bold());
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