use std::time::SystemTime;
use std::{borrow::Cow, env, fs, path::PathBuf};

use log::info;
use self_update::cargo_crate_version;
use serde::{Deserialize, Serialize};

use regex::Captures;
use regex::Regex;

use crate::{CFG_OS, CFG_TARGET_ARCH};

fn unix_epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH
}

/// Describes the sprout-config.yaml file, which stores information on how the current user has configured Sprout.
#[derive(Serialize, Deserialize)]
pub struct SproutConfig {
    pub stash_key: String,
    pub default_repo: String,
    #[serde(default = "unix_epoch")]
    pub last_update_check: SystemTime,
    #[serde(default)]
    pub update_available: Option<String>,
}

/// Represents core Sprout state and helper functions
#[derive(Debug, Clone)]
pub struct Engine {
    pub sprout_home: PathBuf,
}

impl Engine {
    pub fn get_home(&self) -> PathBuf {
        self.sprout_home.clone()
    }

    pub fn get_stash_path(&self) -> PathBuf {
        self.sprout_home.clone().join("stash")
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
                last_update_check: SystemTime::UNIX_EPOCH, // We haven't ever checked!
                update_available: None,
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

    pub fn should_check_for_updates(&self) -> anyhow::Result<bool> {
        let config = self.get_config()?;

        if config.last_update_check.elapsed().unwrap().as_secs() >= 86400 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn check_for_updates(&self) -> anyhow::Result<Option<String>> {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner("talss89")
            .repo_name("sprout")
            .with_target(&format!("{}-{}", CFG_OS, CFG_TARGET_ARCH))
            .build()?
            .fetch()?;

        Ok(match releases.first() {
            Some(release) => {
                if self_update::version::bump_is_greater(cargo_crate_version!(), &release.version)?
                {
                    Some(release.version.clone())
                } else {
                    None
                }
            }
            None => None,
        })
    }

    pub fn get_update_version(&self) -> anyhow::Result<Option<String>> {
        let mut config = self.get_config()?;

        if self.should_check_for_updates()? {
            config.update_available = self.check_for_updates()?;
            config.last_update_check = SystemTime::now();
            self.write_config(&config)?;
        }

        Ok(config.update_available)
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
