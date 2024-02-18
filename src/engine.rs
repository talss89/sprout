use std::{borrow::Cow, env, fs, path::PathBuf};

use log::info;
use serde::{Deserialize, Serialize};

use regex::Captures;
use regex::Regex;

#[derive(Serialize, Deserialize)]
pub struct SproutConfig {
    pub stash_key: String,
    pub default_repo: String,
}

#[derive(Debug, Clone)]
pub struct Engine {
    pub home_path: PathBuf,
}

impl Engine {
    pub fn get_home(&self) -> PathBuf {
        self.home_path.clone()
    }

    pub fn ensure_home(&self) -> anyhow::Result<()> {
        let sprout_home = self.get_home();

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
            fs::create_dir(sprout_home.join("repos"))?;
        }

        if !&sprout_home.join("sprout-config.yaml").exists() {
            self.write_config(&SproutConfig {
                stash_key: "".to_string(),
                default_repo: "".to_string(),
            })?;
        }

        Ok(())
    }
    pub fn get_config(&self) -> anyhow::Result<SproutConfig> {
        let sprout_home = self.get_home();
        Ok(serde_yaml::from_str(&fs::read_to_string(
            sprout_home.join("sprout-config.yaml"),
        )?)?)
    }

    pub fn write_config(&self, config: &SproutConfig) -> anyhow::Result<()> {
        Ok(fs::write(
            self.get_home().join("sprout-config.yaml"),
            serde_yaml::to_string(config)?,
        )?)
    }
}

// (c) Joe_Jingyu - https://stackoverflow.com/questions/62888154/rust-load-environment-variables-into-log4rs-yml-file
pub fn expand_var(raw_config: &str) -> Cow<str> {
    let re = Regex::new(r"\$\{([a-zA-Z_][0-9a-zA-Z_]*)\}").unwrap();
    re.replace_all(&raw_config, |caps: &Captures| match env::var(&caps[1]) {
        Ok(val) => val,
        Err(_) => (&caps[0]).to_string(),
    })
}
