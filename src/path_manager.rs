use std::path::{Path, PathBuf};
use std::env;
use anyhow::{Result, Context};

pub struct PathManager {
    initial_dir: PathBuf,
    current_dir: PathBuf,
}

impl PathManager {
    pub fn new() -> Result<Self> {
        let current = env::current_dir()?;
        Ok(Self {
            initial_dir: current.clone(),
            current_dir: current,
        })
    }

    pub fn change_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let new_path = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.current_dir.join(path)
        };

        if !new_path.exists() {
            std::fs::create_dir_all(&new_path)
                .context("Failed to create directory")?;
        }

        env::set_current_dir(&new_path)
            .context("Failed to change directory")?;
        
        self.current_dir = new_path;
        Ok(())
    }

    pub fn restore_initial_directory(&mut self) -> Result<()> {
        env::set_current_dir(&self.initial_dir)
            .context("Failed to restore initial directory")?;
        self.current_dir = self.initial_dir.clone();
        Ok(())
    }

    pub fn get_current_dir(&self) -> &Path {
        &self.current_dir
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.current_dir.join(path)
        }
    }

    pub fn update_current_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let new_path = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.current_dir.join(path)
        };

        if !new_path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", new_path));
        }

        self.current_dir = new_path;
        Ok(())
    }
} 