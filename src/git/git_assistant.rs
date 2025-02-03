use crate::executor::CommandOutput;
use anyhow::{anyhow, Result};
use git2::{BranchType, MergeAnalysis, MergePreference, Repository};
use std::path::PathBuf;

pub struct GitAssistant {
    repo: Repository,
    ai_client: crate::ai::AIClient,
}

// ... rest of the GitAssistant implementation ...
