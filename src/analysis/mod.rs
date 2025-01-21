use crate::ai;
use crate::config::Config;
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod dependencies;
mod framework;
mod language;

pub use dependencies::DependencyAnalyzer;
pub use framework::FrameworkDetector;
pub use language::LanguageDetector;

#[derive(Debug, Serialize)]
pub struct ProjectAnalysis {
    pub languages: Vec<Language>,
    pub frameworks: Vec<Framework>,
    pub dependencies: Vec<Dependency>,
    pub structure: ProjectStructure,
    pub config_files: Vec<ConfigFile>,
    pub llm_insights: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Language {
    pub name: String,
    pub files: Vec<PathBuf>,
    pub percentage: f32,
    pub loc: usize,
}

#[derive(Debug, Serialize)]
pub struct Framework {
    pub name: String,
    pub version: Option<String>,
    pub language: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub is_dev: bool,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectStructure {
    pub root: PathBuf,
    pub directories: Vec<Directory>,
    pub total_files: usize,
    pub total_size: u64,
}

#[derive(Debug, Serialize)]
pub struct Directory {
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
    pub subdirs: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub file_type: String,
    pub content: serde_json::Value,
}

pub struct ProjectAnalyzer {
    root_path: PathBuf,
    language_detector: LanguageDetector,
    framework_detector: FrameworkDetector,
    dependency_analyzer: DependencyAnalyzer,
}

impl ProjectAnalyzer {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root_path: path.as_ref().to_path_buf(),
            language_detector: LanguageDetector::new(),
            framework_detector: FrameworkDetector::new(),
            dependency_analyzer: DependencyAnalyzer::new(),
        }
    }

    pub async fn analyze(&self) -> Result<ProjectAnalysis> {
        let languages = self.language_detector.detect(&self.root_path).await?;
        let frameworks = self.framework_detector.detect(&self.root_path).await?;
        let dependencies = self.dependency_analyzer.analyze(&self.root_path).await?;
        let structure = self.analyze_structure()?;
        let config_files = self.find_config_files()?;

        Ok(ProjectAnalysis {
            languages,
            frameworks,
            dependencies,
            structure,
            config_files,
            llm_insights: None,
        })
    }

    pub async fn analyze_with_llm(&self, config: &Config) -> Result<ProjectAnalysis> {
        // Get basic analysis first
        let mut analysis = self.analyze().await?;

        // Get LLM insights
        let prompt = format!(
            "Analyze this project:\n\nLanguages:\n{}\n\nFrameworks:\n{}\n\nDependencies:\n{}\n\nProvide insights about:\n1. Project type and purpose\n2. Architecture patterns\n3. Potential improvements\n4. Security considerations",
            analysis.languages.iter().map(|l| format!("- {} ({:.1}%)", l.name, l.percentage)).collect::<Vec<_>>().join("\n"),
            analysis.frameworks.iter().map(|f| format!("- {} {}", f.name, f.version.as_deref().unwrap_or("unknown"))).collect::<Vec<_>>().join("\n"),
            analysis.dependencies.iter().map(|d| format!("- {} {}", d.name, d.version)).collect::<Vec<_>>().join("\n")
        );

        match ai::get_analysis(&prompt, config).await {
            Ok(insights) => {
                analysis.llm_insights = Some(insights);
            }
            Err(e) => {
                eprintln!("Warning: LLM analysis failed: {}", e);
            }
        }

        Ok(analysis)
    }

    fn analyze_structure(&self) -> Result<ProjectStructure> {
        let mut directories = Vec::new();
        let mut total_files = 0;
        let mut total_size = 0;

        for entry in WalkDir::new(&self.root_path) {
            let entry = entry?;
            let path = entry.path().to_path_buf();

            if entry.file_type().is_dir() {
                let files = std::fs::read_dir(&path)?
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
                    .map(|e| e.path())
                    .collect();

                let subdirs = std::fs::read_dir(&path)?
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .map(|e| e.path())
                    .collect();

                directories.push(Directory {
                    path,
                    files,
                    subdirs,
                });
            } else {
                total_files += 1;
                total_size += entry.metadata()?.len();
            }
        }

        Ok(ProjectStructure {
            root: self.root_path.clone(),
            directories,
            total_files,
            total_size,
        })
    }

    fn find_config_files(&self) -> Result<Vec<ConfigFile>> {
        let mut configs = Vec::new();
        let config_patterns = [
            "*.json",
            "*.toml",
            "*.yaml",
            "*.yml",
            "Cargo.toml",
            "package.json",
            "pyproject.toml",
            ".gitignore",
            ".env*",
        ];

        for entry in WalkDir::new(&self.root_path) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            if config_patterns
                .iter()
                .any(|pattern| match glob::Pattern::new(pattern) {
                    Ok(glob) => glob.matches_path(path),
                    Err(_) => false,
                })
            {
                let content = std::fs::read_to_string(path)?;
                let file_type = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content_value: serde_json::Value = match file_type.as_str() {
                    "json" => serde_json::from_str(&content)?,
                    "toml" => {
                        let toml_value: toml::Value = toml::from_str(&content)?;
                        serde_json::to_value(toml_value)?
                    }
                    "yaml" | "yml" => {
                        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)?;
                        serde_json::to_value(yaml_value)?
                    }
                    _ => serde_json::Value::String(content),
                };

                configs.push(ConfigFile {
                    path: path.to_path_buf(),
                    file_type,
                    content: content_value,
                });
            }
        }

        Ok(configs)
    }
}
