use crate::ai::{CommandChain, CommandStep, ResourceImpact};
use crate::shell::ShellType;
use std::time::Duration;

mod command_chain;
mod error_handling;
mod integration;
mod performance;

// Test utilities and helpers
pub(crate) struct TestUtils;

impl TestUtils {
    pub fn create_test_command_chain() -> CommandChain {
        CommandChain {
            steps: vec![CommandStep {
                command: "ls -la".to_string(),
                explanation: "List directory contents".to_string(),
                is_dangerous: false,
                impact: ResourceImpact {
                    cpu_usage: 0.1,
                    memory_usage: 5.0,
                    disk_usage: 0.0,
                    network_usage: 0.0,
                    estimated_duration: Duration::from_secs(1),
                },
                rollback_command: None,
            }],
            total_impact: ResourceImpact {
                cpu_usage: 0.1,
                memory_usage: 5.0,
                disk_usage: 0.0,
                network_usage: 0.0,
                estimated_duration: Duration::from_secs(1),
            },
            explanation: "Test command chain".to_string(),
        }
    }
}
