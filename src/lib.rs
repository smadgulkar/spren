pub mod ai;
pub mod config;
pub mod executor;
pub mod shell;
pub mod intent;
pub mod path_manager;
pub mod git;
pub mod code;

// Re-export commonly used types
pub use ai::{CommandChain, CommandStep};
pub use executor::chain::ChainExecutor;
pub use config::Config;
pub use shell::ShellType;
pub use git::GitManager;
pub use code::CodeGenerator; 