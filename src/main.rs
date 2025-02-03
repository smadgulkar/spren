use anyhow::{Result, anyhow};
use colored::*;
use std::io::{self, Write};
use std::time::Duration;
use crate::executor::chain::ChainExecutor;
use crate::ai::AIError;
use crate::git::GitManager;
use crate::intent::{Intent, IntentAnalyzer, GitOp};

mod ai;
mod config;
mod executor;
mod shell;
mod intent;
mod path_manager;
mod code;
mod git;

use code::CodeGenerator;

#[tokio::main]
async fn main() -> Result<()> {
    // Load or create config
    let config_path = config::get_config_path()?;
    
    println!("Config path: {:?}", config_path); // Add this line for debugging
    
    if !config_path.exists() {
        config::Config::create_default(&config_path)?;
        println!("Created default config file at {:?}", config_path);
        println!("Please update the API key in the config file and restart.");
        return Ok(());
    }

    let config = config::Config::load(&config_path)?;
    
    // Validate API key exists
    if config.ai.anthropic_api_key.is_none() && config.ai.openai_api_key.is_none() {
        println!("No API key found in config file at {:?}", config_path);
        println!("Please add your Anthropic or OpenAI API key to the config file.");
        return Ok(());
    }

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
    let intent = IntentAnalyzer::analyze(query, config).await?;

    match intent {
        Intent::CommandChain => {
            let chain = match ai::get_command_chain(query, &config).await {
                Ok(chain) => chain,
                Err(e) => {
                    match e {
                        AIError::RateLimitError(_msg) => {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            ai::get_command_chain(query, &config).await.map_err(|e| anyhow!(e.to_string()))?
                        }
                        AIError::NetworkError(_msg) => {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            ai::get_command_chain(query, &config).await.map_err(|e| anyhow!(e.to_string()))?
                        }
                        _ => return Err(anyhow!(e.to_string()))
                    }
                }
            };

            let mut executor = ChainExecutor::new(chain)?;

            // Show preview
            println!("\n{}", "Command Chain Preview:".blue().bold());
            println!("{}", executor.preview());

            if config.security.require_confirmation {
                print!("\nExecute this command chain? [y/N/s(step-by-step)] ");
                io::stdout().flush()?;

                let mut response = String::new();
                io::stdin().read_line(&mut response)?;
                
                match response.trim().to_lowercase().as_str() {
                    "y" | "yes" => {
                        match executor.execute_all().await {
                            Ok(outputs) => {
                                for output in outputs {
                                    if !output.stdout.is_empty() {
                                        println!("{}", output.stdout);
                                    }
                                    if !output.stderr.is_empty() {
                                        eprintln!("{}", output.stderr.red());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Chain execution failed: {}", e);
                                print!("\nWould you like to rollback the changes? [y/N] ");
                                io::stdout().flush()?;
                                
                                let mut response = String::new();
                                io::stdin().read_line(&mut response)?;
                                
                                if response.trim().to_lowercase() == "y" {
                                    match executor.rollback().await {
                                        Ok(_) => println!("Rollback successful"),
                                        Err(e) => eprintln!("Rollback failed: {}", e),
                                    }
                                }
                            }
                        }
                    }
                    "s" => {
                        while !executor.is_complete() {
                            if let Some(step) = executor.current_step_details() {
                                let (current, total) = executor.progress();
                                println!("\nStep {} of {}: {}", current + 1, total, step.explanation);
                                println!("Command: {}", step.command);
                                
                                print!("Execute this step? [y/N/s(skip)] ");
                                io::stdout().flush()?;
                                
                                let mut response = String::new();
                                io::stdin().read_line(&mut response)?;
                                
                                match response.trim().to_lowercase().as_str() {
                                    "y" | "yes" => {
                                        match executor.execute_next().await {
                                            Ok(Some(output)) => {
                                                if !output.stdout.is_empty() {
                                                    println!("{}", output.stdout);
                                                }
                                                if !output.stderr.is_empty() {
                                                    eprintln!("{}", output.stderr.red());
                                                }
                                            }
                                            Ok(None) => break,
                                            Err(e) => {
                                                eprintln!("Step failed: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                    "s" | "skip" => {
                                        executor.skip_step()?;
                                    }
                                    _ => {
                                        println!("Chain execution cancelled");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Chain execution cancelled");
                        return Ok(());
                    }
                }
            }
        }
        Intent::CodeGeneration(code_intent) => {
            println!("{}", "Generating code...".yellow().bold());
            println!("Language: {:?}", code_intent.language);
            println!("Description: {}", code_intent.description);

            let current_dir = std::env::current_dir()?;
            let extension = code_intent.language.get_extension();
            let generator = CodeGenerator::new(&current_dir, code_intent.language)?;

            // For now, just create a basic file
            let filename = format!("generated_code.{}", extension);
            let path = current_dir.join(filename);

            generator.generate_file(&path, "// Generated code\n// TODO: Implement AI code generation")?;
            println!("Generated file at: {}", path.display());
        }
        Intent::GitOperation(git_intent) => {
            println!("{}", "Executing git operation...".yellow().bold());
            
            // Create GitManager for current directory
            let current_dir = std::env::current_dir()?;
            let git_manager = GitManager::new(&current_dir)?;

            match git_intent.operation {
                GitOp::Status => {
                    let status = git_manager.get_status()?;
                    println!("\n{}", "Repository Status:".blue().bold());
                    
                    if !status.staged_modified.is_empty() {
                        println!("\n{}", "Modified (staged):".green());
                        for file in status.staged_modified {
                            println!("  {}", file);
                        }
                    }

                    if !status.staged_added.is_empty() {
                        println!("\n{}", "Added (staged):".green());
                        for file in status.staged_added {
                            println!("  {}", file);
                        }
                    }

                    if !status.staged_deleted.is_empty() {
                        println!("\n{}", "Deleted (staged):".yellow());
                        for file in status.staged_deleted {
                            println!("  {}", file);
                        }
                    }

                    if !status.unstaged_modified.is_empty() {
                        println!("\n{}", "Modified (unstaged):".red());
                        for file in status.unstaged_modified {
                            println!("  {}", file);
                        }
                    }

                    if !status.untracked.is_empty() {
                        println!("\n{}", "Untracked:".red().bold());
                        for file in status.untracked {
                            println!("  {}", file);
                        }
                    }
                },
                GitOp::Branch => {
                    println!("Current branch: {}", git_manager.get_current_branch()?.green());
                },
                GitOp::Commit => {
                    println!("Commit operation not implemented yet");
                },
                GitOp::ListBranches => {
                    let branches = git_manager.list_branches()?;
                    println!("\n{}", "Available branches:".blue().bold());
                    for branch in branches {
                        if branch == git_manager.get_current_branch()? {
                            println!("* {}", branch.green());
                        } else {
                            println!("  {}", branch);
                        }
                    }
                },
                GitOp::SwitchBranch => {
                    // Extract branch name from args
                    if let Some(branch_name) = git_intent.args.iter()
                        .skip_while(|&arg| !arg.contains("branch"))
                        .nth(1) {
                        git_manager.switch_branch(branch_name)?;
                        println!("Switched to branch: {}", branch_name.green());
                    } else {
                        println!("Please specify a branch name");
                    }
                },
                GitOp::CreateBranch => {
                    // Extract branch name from args
                    if let Some(branch_name) = git_intent.args.iter()
                        .skip_while(|&arg| !arg.contains("branch"))
                        .nth(1) {
                        git_manager.create_branch(branch_name)?;
                        println!("Created and switched to branch: {}", branch_name.green());
                    } else {
                        println!("Please specify a branch name");
                    }
                },
                GitOp::ShowDiff => {
                    // TODO: Implement diff viewing
                    println!("Diff viewing not implemented yet");
                },
                GitOp::Analyze => {
                    println!("Analysis operation not implemented yet");
                },
            }
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