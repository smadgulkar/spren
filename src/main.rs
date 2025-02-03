use crate::executor::chain::ChainExecutor;
use crate::git::GitManager;
use crate::intent::{Intent, IntentExecutor, IntentMatcher};
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

mod ai;
mod code;
mod config;
mod executor;
mod git;
mod intent;
mod path_manager;
mod shell;

use code::CodeGenerator;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = config::get_config_path()?;

    if !config_path.exists() {
        config::Config::create_default(&config_path)?;
        println!("Created default config file at {:?}", config_path);
        println!("Please update the API key in the config file and restart.");
        return Ok(());
    }

    let config = config::Config::load(&config_path)?;
    let executor = IntentExecutor::new(config.clone());
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

        let intent = IntentMatcher::analyze(query, &config).await?;
        let result = executor.execute(&intent).await?;

        if result.success {
            println!("{}", result.output);
        } else {
            eprintln!(
                "{}: {}",
                "Error".red().bold(),
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }
    }

    Ok(())
}
