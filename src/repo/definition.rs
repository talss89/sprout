use capturing_glob::glob;
use colored::*;
use rustic_backend::BackendOptions;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use crate::engine::Engine;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryDefinition {
    pub repo_key: String,

    #[serde(flatten)]
    pub repo: BackendOptions,
}

impl RepositoryDefinition {
    pub fn create(definition: &RepositoryDefinition, path: &PathBuf) -> anyhow::Result<()> {
        if path.exists() {
            return Err(anyhow::anyhow!(
                "A repository definition at already exists with the same label!"
            ));
        }

        Self::save(definition, path)
    }

    pub fn save(definition: &RepositoryDefinition, path: &PathBuf) -> anyhow::Result<()> {
        fs::write(path, serde_yaml::to_string(&definition)?)?;
        Ok(())
    }

    pub fn list(engine: &Engine) -> anyhow::Result<Vec<(String, RepositoryDefinition)>> {
        let mut results = vec![];

        for entry in glob(&format!(
            "{}/repos/(*).yaml",
            engine.get_home().to_string_lossy()
        ))
        .expect("Failed to read glob pattern")
        .flatten()
        {
            let label = entry.group(1).unwrap().to_str().unwrap();
            results.push((String::from(label), Self::get(engine, label)?.1));
        }

        Ok(results)
    }

    pub fn get(engine: &Engine, label: &str) -> anyhow::Result<(PathBuf, RepositoryDefinition)> {
        let path = engine.get_home().join(format!("repos/{}.yaml", label));

        if !path.exists() {
            return Err(anyhow::anyhow!(
                "The repo definition for {} does not exist at {}",
                label,
                path.to_string_lossy()
            ));
        }

        Ok((
            path.to_owned(),
            serde_yaml::from_str(&crate::engine::expand_var(&fs::read_to_string(path)?))?,
        ))
    }

    pub fn display_path(repo: &RepositoryDefinition) -> anyhow::Result<String> {
        Ok(match &repo.repo.repository {
            Some(repository_name) => match repository_name.as_str() {
                "opendal:s3" => {
                    format!(
                        "{} {}",
                        repository_name,
                        format!(
                            "(region: {}, bucket: {}, endpoint: {})",
                            repo.repo
                                .options
                                .get("region")
                                .unwrap_or(&"???".to_string()),
                            repo.repo
                                .options
                                .get("bucket")
                                .unwrap_or(&"???".to_string()),
                            repo.repo
                                .options
                                .get("endpoint")
                                .unwrap_or(&"???".to_string())
                        )
                        .dimmed()
                    )
                }
                _ => repository_name.to_owned(),
            },
            None => "???".to_string(),
        })
    }
}
