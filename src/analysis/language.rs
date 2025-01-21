use anyhow::Result;
use std::path::{Path, PathBuf};
use tokei::{Config, Languages};

pub struct LanguageDetector;

impl LanguageDetector {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect(&self, path: &Path) -> Result<Vec<super::Language>> {
        let mut languages = Languages::new();
        let config = Config::default();
        languages.get_statistics(&[path], &[], &config);

        let mut results = Vec::new();
        let total_code: f64 = languages.iter().map(|(_, stats)| stats.code as f64).sum();

        for (lang_type, stats) in languages.iter() {
            let percentage = if total_code > 0.0 {
                (stats.code as f64 / total_code * 100.0) as f32
            } else {
                0.0
            };

            let files = stats
                .reports
                .iter()
                .map(|report| PathBuf::from(&report.name))
                .collect();

            results.push(super::Language {
                name: lang_type.name().to_string(),
                files,
                percentage,
                loc: stats.code,
            });
        }

        Ok(results)
    }
}
