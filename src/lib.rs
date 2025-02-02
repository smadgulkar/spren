pub mod ai;
pub mod config;
pub mod executor;
pub mod shell;
pub mod intent;
pub mod path_manager;

// Re-export commonly used types
pub use ai::{CommandChain, CommandStep};
pub use executor::chain::ChainExecutor;
pub use config::Config;
pub use shell::ShellType; 