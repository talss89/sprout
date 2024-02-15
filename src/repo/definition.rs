use capturing_glob::glob;
use rustic_backend::BackendOptions;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryDefinition {
    pub access_key: String,

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
        fs::write(path, &serde_yaml::to_string(&definition)?)?;
        Ok(())
    }

    pub fn list() -> anyhow::Result<Vec<(String, RepositoryDefinition)>> {
        let mut results = vec![];

        for entry in glob(&format!(
            "{}/repos/(*).yaml",
            crate::engine::get_sprout_home().to_string_lossy()
        ))
        .expect("Failed to read glob pattern")
        {
            match entry {
                Ok(entry) => {
                    let label = entry.group(1).unwrap().to_str().unwrap();
                    results.push((String::from(label), Self::get(label)?.1));
                }
                _ => {}
            }
        }

        Ok(results)
    }

    pub fn get(label: &str) -> anyhow::Result<(PathBuf, RepositoryDefinition)> {
        let path = crate::engine::get_sprout_home().join(format!("repos/{}.yaml", label));

        if !path.exists() {
            return Err(anyhow::anyhow!(
                "The repo definition for {} does not exist at {}",
                label,
                path.to_string_lossy()
            ));
        }

        Ok((path.to_owned(), serde_yaml::from_slice(&fs::read(path)?)?))
    }
}
