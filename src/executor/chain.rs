use anyhow::{Result, anyhow};
use std::time::{Instant, Duration};
use crate::ai::{CommandChain, CommandStep};
use crate::executor::CommandOutput;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChainStatus {
    NotStarted,
    Running,
    Paused,
    Complete,
    Failed,
}

pub struct ChainExecutor {
    chain: CommandChain,
    current_step: usize,
    start_time: Option<Instant>,
    status: ChainStatus,
    failed_step: Option<usize>,
}

impl ChainExecutor {
    pub fn new(chain: CommandChain) -> Self {
        Self {
            chain,
            current_step: 0,
            start_time: None,
            status: ChainStatus::NotStarted,
            failed_step: None,
        }
    }

    pub fn preview(&self) -> String {
        let mut preview = String::new();
        preview.push_str(&format!("Task: {}\n\n", self.chain.explanation));
        
        for (i, step) in self.chain.steps.iter().enumerate() {
            preview.push_str(&format!("Step {}: {}\n", i + 1, step.explanation));
            preview.push_str(&format!("Command: {}\n", step.command));
            if step.is_dangerous {
                preview.push_str("⚠️  This step is potentially dangerous!\n");
            }
            preview.push_str(&format!("Estimated impact:\n"));
            preview.push_str(&format!("  CPU: {:.1}%\n", step.impact.cpu_usage));
            preview.push_str(&format!("  Memory: {:.1}MB\n", step.impact.memory_usage));
            preview.push_str(&format!("  Disk: {:.1}MB\n", step.impact.disk_usage));
            preview.push_str(&format!("  Network: {:.1}MB\n", step.impact.network_usage));
            preview.push_str(&format!("  Duration: {:?}\n", step.impact.estimated_duration));
            preview.push_str("\n");
        }

        preview.push_str(&format!("Total estimated impact:\n"));
        preview.push_str(&format!("  CPU: {:.1}%\n", self.chain.total_impact.cpu_usage));
        preview.push_str(&format!("  Memory: {:.1}MB\n", self.chain.total_impact.memory_usage));
        preview.push_str(&format!("  Disk: {:.1}MB\n", self.chain.total_impact.disk_usage));
        preview.push_str(&format!("  Network: {:.1}MB\n", self.chain.total_impact.network_usage));
        preview.push_str(&format!("  Total duration: {:?}\n", self.chain.total_impact.estimated_duration));

        preview
    }

    pub async fn execute_next(&mut self) -> Result<Option<CommandOutput>> {
        if self.status == ChainStatus::Failed {
            return Err(anyhow!("Chain execution previously failed at step {}", 
                self.failed_step.unwrap_or(0) + 1));
        }

        if self.current_step >= self.chain.steps.len() {
            self.status = ChainStatus::Complete;
            return Ok(None);
        }

        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
            self.status = ChainStatus::Running;
        }

        let step = &self.chain.steps[self.current_step];
        match super::execute_command(&step.command).await {
            Ok(output) => {
                if !output.success {
                    self.status = ChainStatus::Failed;
                    self.failed_step = Some(self.current_step);
                    return Err(anyhow!("Step {} failed: {}", 
                        self.current_step + 1, output.stderr));
                }
                self.current_step += 1;
                Ok(Some(output))
            },
            Err(e) => {
                self.status = ChainStatus::Failed;
                self.failed_step = Some(self.current_step);
                Err(e)
            }
        }
    }

    pub async fn execute_all(&mut self) -> Result<Vec<CommandOutput>> {
        let mut outputs = Vec::new();
        while let Some(output) = self.execute_next().await? {
            outputs.push(output);
        }
        Ok(outputs)
    }

    pub async fn rollback(&mut self) -> Result<Vec<CommandOutput>> {
        let mut outputs = Vec::new();
        
        while self.current_step > 0 {
            self.current_step -= 1;
            if let Some(rollback_cmd) = &self.chain.steps[self.current_step].rollback_command {
                match super::execute_command(rollback_cmd).await {
                    Ok(output) => outputs.push(output),
                    Err(e) => return Err(anyhow!("Rollback failed at step {}: {}", 
                        self.current_step + 1, e)),
                }
            }
        }

        self.status = ChainStatus::NotStarted;
        self.failed_step = None;
        Ok(outputs)
    }

    pub fn pause(&mut self) {
        if self.status == ChainStatus::Running {
            self.status = ChainStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == ChainStatus::Paused {
            self.status = ChainStatus::Running;
        }
    }

    pub fn skip_step(&mut self) -> Result<()> {
        if self.current_step < self.chain.steps.len() {
            self.current_step += 1;
            Ok(())
        } else {
            Err(anyhow!("No more steps to skip"))
        }
    }

    pub fn status(&self) -> ChainStatus {
        self.status
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.current_step, self.chain.steps.len())
    }

    pub fn is_complete(&self) -> bool {
        self.status == ChainStatus::Complete
    }

    pub fn elapsed_time(&self) -> Option<Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    pub fn current_step_details(&self) -> Option<&CommandStep> {
        if self.current_step < self.chain.steps.len() {
            Some(&self.chain.steps[self.current_step])
        } else {
            None
        }
    }
} 