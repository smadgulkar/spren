use super::Intent;
use crate::ai::{CommandChain, CommandStep, ResourceImpact};
use crate::config::Config;
use crate::executor::chain::ChainExecutor;
use crate::git::GitManager;
use anyhow::Result;
use std::time::Duration;

#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct IntentExecutor {
    config: Config,
}

impl IntentExecutor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn execute(&self, intent: &Intent) -> Result<ExecutionResult> {
        match intent {
            Intent::GitOperation(params) => {
                let git_manager = GitManager::new(&std::env::current_dir()?, self.config.clone())?;
                let output = git_manager.execute(&params.operation).await?;
                Ok(ExecutionResult {
                    success: true,
                    output,
                    error: None,
                })
            }
            Intent::CommandChain(params) => {
                let steps: Vec<CommandStep> = params
                    .commands
                    .iter()
                    .map(|cmd| CommandStep {
                        command: cmd.clone(),
                        explanation: params.explanation.clone(),
                        is_dangerous: false,
                        impact: ResourceImpact {
                            cpu_usage: 0.1,
                            memory_usage: 1.0,
                            disk_usage: 0.0,
                            network_usage: 0.0,
                            estimated_duration: Duration::from_secs_f32(0.1),
                        },
                        rollback_command: None,
                    })
                    .collect();

                let chain = CommandChain {
                    steps,
                    total_impact: ResourceImpact::default(),
                    explanation: params.explanation.clone(),
                };

                let mut executor = ChainExecutor::new(chain)?;
                let output = executor.execute_all().await?;

                let output_strings: Vec<String> = output
                    .into_iter()
                    .map(|cmd_output| cmd_output.to_string())
                    .collect();

                Ok(ExecutionResult {
                    success: true,
                    output: output_strings.join("\n"),
                    error: None,
                })
            }
            Intent::CodeGeneration(_) => Ok(ExecutionResult {
                success: true,
                output: "Code generation not implemented yet".to_string(),
                error: None,
            }),
            Intent::Unknown => Ok(ExecutionResult {
                success: false,
                output: "Unknown intent".to_string(),
                error: Some("Could not determine intent".to_string()),
            }),
        }
    }
}
