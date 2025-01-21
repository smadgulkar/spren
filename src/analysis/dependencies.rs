use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use toml::Value as TomlValue;

pub struct DependencyAnalyzer {
    dependency_parsers: HashMap<String, Box<dyn DependencyParser + Send + Sync>>,
}

#[async_trait::async_trait]
trait DependencyParser {
    async fn parse_dependencies(&self, path: &Path) -> Result<Vec<super::Dependency>>;
    fn get_file_pattern(&self) -> &str;
}

struct RustDependencyParser;
struct NodeDependencyParser;
struct PythonDependencyParser;

#[async_trait::async_trait]
impl DependencyParser for RustDependencyParser {
    fn get_file_pattern(&self) -> &str {
        "Cargo.toml"
    }

    async fn parse_dependencies(&self, path: &Path) -> Result<Vec<super::Dependency>> {
        let cargo_path = path.join(self.get_file_pattern());
        if !cargo_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(cargo_path)?;
        let cargo_toml: TomlValue = toml::from_str(&content)?;

        let mut deps = Vec::new();

        // Parse regular dependencies
        if let Some(dependencies) = cargo_toml.get("dependencies").and_then(|d| d.as_table()) {
            for (name, version) in dependencies {
                let version_str = match version {
                    TomlValue::String(v) => v.clone(),
                    TomlValue::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };

                deps.push(super::Dependency {
                    name: name.clone(),
                    version: version_str,
                    is_dev: false,
                    source: "crates.io".to_string(),
                });
            }
        }

        // Parse dev-dependencies
        if let Some(dev_deps) = cargo_toml
            .get("dev-dependencies")
            .and_then(|d| d.as_table())
        {
            for (name, version) in dev_deps {
                let version_str = match version {
                    TomlValue::String(v) => v.clone(),
                    TomlValue::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };

                deps.push(super::Dependency {
                    name: name.clone(),
                    version: version_str,
                    is_dev: true,
                    source: "crates.io".to_string(),
                });
            }
        }

        Ok(deps)
    }
}

#[async_trait::async_trait]
impl DependencyParser for NodeDependencyParser {
    fn get_file_pattern(&self) -> &str {
        "package.json"
    }

    async fn parse_dependencies(&self, path: &Path) -> Result<Vec<super::Dependency>> {
        let package_path = path.join(self.get_file_pattern());
        if !package_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(package_path)?;
        let package_json: JsonValue = serde_json::from_str(&content)?;

        let mut deps = Vec::new();

        // Parse regular dependencies
        if let Some(dependencies) = package_json.get("dependencies").and_then(|d| d.as_object()) {
            for (name, version) in dependencies {
                deps.push(super::Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    is_dev: false,
                    source: "npm".to_string(),
                });
            }
        }

        // Parse dev dependencies
        if let Some(dev_deps) = package_json
            .get("devDependencies")
            .and_then(|d| d.as_object())
        {
            for (name, version) in dev_deps {
                deps.push(super::Dependency {
                    name: name.clone(),
                    version: version.as_str().unwrap_or("*").to_string(),
                    is_dev: true,
                    source: "npm".to_string(),
                });
            }
        }

        Ok(deps)
    }
}

#[async_trait::async_trait]
impl DependencyParser for PythonDependencyParser {
    fn get_file_pattern(&self) -> &str {
        "requirements.txt"
    }

    async fn parse_dependencies(&self, path: &Path) -> Result<Vec<super::Dependency>> {
        let mut deps = Vec::new();

        // Check requirements.txt
        let req_path = path.join(self.get_file_pattern());
        if req_path.exists() {
            let content = std::fs::read_to_string(req_path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let parts: Vec<&str> = line.splitn(2, '=').collect();
                let name = parts[0].trim();
                let version = parts.get(1).map(|v| *v).unwrap_or("*");

                deps.push(super::Dependency {
                    name: name.to_string(),
                    version: version.to_string(),
                    is_dev: false,
                    source: "pip".to_string(),
                });
            }
        }

        // Check pyproject.toml
        let pyproject_path = path.join("pyproject.toml");
        if pyproject_path.exists() {
            let content = std::fs::read_to_string(pyproject_path)?;
            if let Ok(pyproject) = toml::from_str::<TomlValue>(&content) {
                if let Some(poetry) = pyproject.get("tool").and_then(|t| t.get("poetry")) {
                    if let Some(deps_table) = poetry.get("dependencies").and_then(|d| d.as_table())
                    {
                        for (name, version) in deps_table {
                            deps.push(super::Dependency {
                                name: name.clone(),
                                version: version.as_str().unwrap_or("*").to_string(),
                                is_dev: false,
                                source: "poetry".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(deps)
    }
}

impl DependencyAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            dependency_parsers: HashMap::new(),
        };

        analyzer.dependency_parsers.insert(
            "rust".to_string(),
            Box::new(RustDependencyParser) as Box<dyn DependencyParser + Send + Sync>,
        );
        analyzer.dependency_parsers.insert(
            "node".to_string(),
            Box::new(NodeDependencyParser) as Box<dyn DependencyParser + Send + Sync>,
        );
        analyzer.dependency_parsers.insert(
            "python".to_string(),
            Box::new(PythonDependencyParser) as Box<dyn DependencyParser + Send + Sync>,
        );

        analyzer
    }

    pub async fn analyze(&self, path: &Path) -> Result<Vec<super::Dependency>> {
        let mut all_deps = Vec::new();

        for parser in self.dependency_parsers.values() {
            let deps = parser.parse_dependencies(path).await?;
            all_deps.extend(deps);
        }

        Ok(all_deps)
    }
}
