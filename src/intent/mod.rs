mod executor;
mod matcher;
mod parameters;

pub use self::executor::{ExecutionResult, IntentExecutor};
pub use self::matcher::IntentMatcher;
pub use self::parameters::*;

#[derive(Debug, PartialEq)]
pub enum Intent {
    CommandChain(CommandChainParams),
    GitOperation(GitParams),
    CodeGeneration(CodeGenParams),
    Unknown,
}
