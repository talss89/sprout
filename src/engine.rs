use std::{fs, path::PathBuf};

use homedir::get_my_home;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SproutConfig {
    pub stash_key: String,
    pub default_repo: String,
}

pub fn get_sprout_home() -> PathBuf {
    get_my_home().unwrap().unwrap().as_path().join(".sprout")
}

pub fn ensure_sprout_home() -> anyhow::Result<()> {
    let sprout_home = get_sprout_home();

    if !&sprout_home.exists() {
        info!(
            "Sprout home directory {} doesn't exist. Creating...",
            &sprout_home.to_string_lossy()
        );
        fs::create_dir(&sprout_home)?;
    }

    if !&sprout_home.join("repos").exists() {
        info!(
            "Sprout home directory {} doesn't exist. Creating...",
            &sprout_home.join("repos").to_string_lossy()
        );
        fs::create_dir(&sprout_home.join("repos"))?;
    }

    if !&sprout_home.join("sprout-config.yaml").exists() {
        write_sprout_config(&SproutConfig {
            stash_key: "".to_string(),
            default_repo: "".to_string(),
        })?;
    }

    Ok(())
}
pub fn get_sprout_config() -> anyhow::Result<SproutConfig> {
    let sprout_home = get_sprout_home();
    Ok(serde_yaml::from_str(&fs::read_to_string(
        sprout_home.join("sprout-config.yaml"),
    )?)?)
}

pub fn write_sprout_config(config: &SproutConfig) -> anyhow::Result<()> {
    Ok(fs::write(
        get_sprout_home().join("sprout-config.yaml"),
        serde_yaml::to_string(config)?,
    )?)
}
