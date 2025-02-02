use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CodeGenerator {
    project_root: PathBuf,
    language: Language,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Unknown,
    // Add more as needed
}

impl CodeGenerator {
    pub fn new(project_root: impl AsRef<Path>, language: Language) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        if !project_root.exists() {
            return Err(anyhow::anyhow!("Project root does not exist: {:?}", project_root));
        }
        Ok(Self { project_root, language })
    }

    pub fn analyze_project(&self) -> Result<ProjectStructure> {
        let mut structure = ProjectStructure::default();
        self.scan_directory(&self.project_root, &mut structure)?;
        Ok(structure)
    }

    fn scan_directory(&self, dir: &Path, structure: &mut ProjectStructure) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if !should_ignore(&path) {
                    structure.directories.push(path.to_string_lossy().into_owned());
                    self.scan_directory(&path, structure)?;
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if is_source_file(ext) {
                        structure.source_files.push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_language(&self) -> &Language {
        &self.language
    }

    pub fn generate_file(&self, path: &Path, content: &str) -> Result<()> {
        let content = match self.language {
            Language::Rust => format!("//! {}\n\n{}", path.display(), content),
            Language::Python => format!("# {}\n\n{}", path.display(), content),
            Language::JavaScript | Language::TypeScript => {
                format!("/**\n * {}\n */\n\n{}", path.display(), content)
            }
            Language::Unknown => content.to_string(),
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct ProjectStructure {
    pub directories: Vec<String>,
    pub source_files: Vec<String>,
    pub dependencies: Vec<String>,
}

fn should_ignore(path: &Path) -> bool {
    let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    matches!(name, 
        "target" | "node_modules" | ".git" | 
        "dist" | "build" | ".idea" | ".vscode"
    )
}

fn is_source_file(ext: &std::ffi::OsStr) -> bool {
    matches!(ext.to_str().unwrap_or(""),
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" |
        "java" | "cpp" | "c" | "h" | "hpp"
    )
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" | "jsx" => Language::JavaScript,
            "ts" | "tsx" => Language::TypeScript,
            _ => Language::Unknown,
        }
    }

    pub fn get_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::Unknown => "txt",
        }
    }
} 