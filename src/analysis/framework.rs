use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use toml::Value as TomlValue;

pub struct FrameworkDetector {
    framework_patterns: HashMap<String, Vec<FrameworkPattern>>,
}

struct FrameworkPattern {
    file_pattern: String,
    content_match: Box<dyn Fn(&str) -> bool + Send + Sync>,
    version_extract: Box<dyn Fn(&str) -> Option<String> + Send + Sync>,
}

impl FrameworkDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            framework_patterns: HashMap::new(),
        };
        detector.initialize_patterns();
        detector
    }

    fn initialize_patterns(&mut self) {
        // Rust patterns
        self.add_framework_patterns(
            "Rust",
            vec![
                (
                    "Cargo.toml",
                    Box::new(|content| content.contains("[dependencies]")),
                    Box::new(|content| {
                        if let Ok(toml) = toml::from_str::<TomlValue>(content) {
                            toml.get("package")?
                                .get("version")?
                                .as_str()
                                .map(String::from)
                        } else {
                            None
                        }
                    }),
                ),
                (
                    "src/lib.rs",
                    Box::new(|content| {
                        content.contains("#[macro_use]") || content.contains("pub mod")
                    }),
                    Box::new(|_| None),
                ),
            ],
        );

        // Node.js patterns
        self.add_framework_patterns(
            "Node.js",
            vec![(
                "package.json",
                Box::new(|content| {
                    content.contains("\"dependencies\"") || content.contains("\"devDependencies\"")
                }),
                Box::new(|content| {
                    serde_json::from_str::<JsonValue>(content)
                        .ok()
                        .and_then(|v| v.get("version")?.as_str().map(String::from))
                }),
            )],
        );

        // Python patterns
        self.add_framework_patterns(
            "Python",
            vec![
                ("requirements.txt", Box::new(|_| true), Box::new(|_| None)),
                (
                    "pyproject.toml",
                    Box::new(|content| {
                        content.contains("[tool.poetry]") || content.contains("[build-system]")
                    }),
                    Box::new(|content| {
                        toml::from_str::<TomlValue>(content).ok().and_then(|v| {
                            v.get("tool")?
                                .get("poetry")?
                                .get("version")?
                                .as_str()
                                .map(String::from)
                        })
                    }),
                ),
            ],
        );
    }

    fn add_framework_patterns(
        &mut self,
        language: &str,
        patterns: Vec<(
            &str,
            Box<dyn Fn(&str) -> bool + Send + Sync>,
            Box<dyn Fn(&str) -> Option<String> + Send + Sync>,
        )>,
    ) {
        let framework_patterns = patterns
            .into_iter()
            .map(|(file, matcher, version_extractor)| FrameworkPattern {
                file_pattern: file.to_string(),
                content_match: matcher,
                version_extract: version_extractor,
            })
            .collect();

        self.framework_patterns
            .insert(language.to_string(), framework_patterns);
    }

    pub async fn detect(&self, path: &Path) -> Result<Vec<super::Framework>> {
        let mut frameworks = Vec::new();

        for (language, patterns) in &self.framework_patterns {
            for pattern in patterns {
                let file_path = path.join(&pattern.file_pattern);
                if file_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if (pattern.content_match)(&content) {
                            let version = (pattern.version_extract)(&content);
                            frameworks.push(super::Framework {
                                name: self
                                    .determine_framework_name(language, &pattern.file_pattern),
                                version,
                                language: language.clone(),
                                confidence: self.calculate_confidence(&file_path, &content),
                            });
                        }
                    }
                }
            }
        }

        Ok(frameworks)
    }

    fn determine_framework_name(&self, language: &str, file_pattern: &str) -> String {
        match (language, file_pattern) {
            ("Rust", "Cargo.toml") => "Rust/Cargo".to_string(),
            ("Node.js", "package.json") => {
                if file_pattern.contains("angular.json") {
                    "Angular".to_string()
                } else if file_pattern.contains("react") {
                    "React".to_string()
                } else {
                    "Node.js".to_string()
                }
            }
            ("Python", "pyproject.toml") => "Poetry".to_string(),
            ("Python", "requirements.txt") => "pip".to_string(),
            _ => language.to_string(),
        }
    }

    fn calculate_confidence(&self, path: &Path, content: &str) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Increase confidence based on file existence
        if path.exists() {
            confidence += 0.3;
        }

        // Adjust based on content analysis
        if !content.is_empty() {
            confidence += 0.2;
        }

        confidence
    }
}
