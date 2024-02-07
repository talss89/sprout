use std::{fs, path::PathBuf};

use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{
    repofile::SnapshotFile, ConfigOptions, KeyOptions, Repository, RepositoryOptions,
};
use serde::{Deserialize, Serialize};

use crate::{project::Project, repo::SproutProgressBar};

pub struct Stash {
    pub path: PathBuf,
    pub config: StashConfig,
}

#[derive(Serialize, Deserialize)]
pub struct StashConfig {
    pub key: String,
}

impl Stash {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            warn!("Stash does not exist at {}.", path.to_string_lossy());

            Stash::initialise(path.to_owned())?;
        }

        let config = serde_yaml::from_str(&fs::read_to_string(path.join("stash-config.yaml"))?)?;

        Ok(Self { path, config })
    }

    pub fn initialise(path: PathBuf) -> anyhow::Result<()> {
        if path.join("stash-config.yaml").exists() {
            return Err(anyhow::anyhow!(
                "A stash configuration already exists at {}",
                path.to_string_lossy()
            ));
        }

        info!("Initialising new stash at {}", path.to_string_lossy());

        let pg = PasswordGenerator::new()
            .length(32)
            .numbers(true)
            .lowercase_letters(true)
            .uppercase_letters(true)
            .symbols(false)
            .spaces(false)
            .strict(true);

        let passkey = pg.generate_one().unwrap();

        let backend = BackendOptions::default().repository(path.join("stash").to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(&passkey);

        let repo = crate::repo::open_repo(&backend, repo_opts)?;

        let key_opts = KeyOptions::default();
        let config_opts = ConfigOptions::default();
        let _repo = repo.init(&key_opts, &config_opts)?;

        let stash_config = StashConfig {
            key: passkey.clone(),
        };

        fs::write(
            path.join("stash-config.yaml"),
            serde_yaml::to_string(&stash_config)?,
        )?;

        Ok(())
    }

    fn open_stash(&self) -> anyhow::Result<Repository<SproutProgressBar, ()>> {
        let backend =
            BackendOptions::default().repository(self.path.join("stash").to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(&self.config.key);

        crate::repo::open_repo(&backend, repo_opts)
    }

    pub fn stash(&self, project: &Project) -> anyhow::Result<()> {
        info!("Stashing {}...", project.config.name);
        let repo = self.open_stash()?;

        let id = crate::repo::snapshot(repo, project)?;

        info!("Stashed with snapshot id {}", id);
        info!(
            "To restore, run `sprout unstash` or `sprout unstash {}`",
            id
        );

        Ok(())
    }

    pub fn restore(&self, project: &Project, snap_id: String) -> anyhow::Result<()> {
        info!("Restoring stash...");
        let repo = self.open_stash()?;

        let _id = crate::repo::restore(repo, project, snap_id)?;

        Ok(())
    }

    pub fn get_latest_stash(&self, project: &Project) -> anyhow::Result<SnapshotFile> {
        let repo = self.open_stash()?;

        let node = repo
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == project.config.name && snap.tags.contains("sprt_obj:database") {
                    return true;
                }

                false
            })?;

        Ok(node)
    }
}
