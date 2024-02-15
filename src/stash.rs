use std::path::PathBuf;

use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{
    repofile::SnapshotFile, ConfigOptions, Id, KeyOptions, Repository, RepositoryOptions,
};

use crate::{engine::*, progress::SproutProgressBar, project::Project};

pub struct Stash {
    pub path: PathBuf,
}

impl Stash {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            warn!("Stash does not exist at {}.", path.to_string_lossy());

            Stash::initialise(path.to_owned())?;
        }

        Ok(Self { path })
    }

    pub fn initialise(path: PathBuf) -> anyhow::Result<()> {
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

        let mut sprout_config = get_sprout_config()?;

        sprout_config.stash_key = passkey;

        write_sprout_config(&sprout_config)?;

        Ok(())
    }

    fn open_stash(&self) -> anyhow::Result<Repository<SproutProgressBar, ()>> {
        let sprout_config = get_sprout_config()?;
        let backend =
            BackendOptions::default().repository(self.path.join("stash").to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(&sprout_config.stash_key);

        crate::repo::open_repo(&backend, repo_opts)
    }

    pub fn stash(&self, project: &Project) -> anyhow::Result<()> {
        info!("Stashing {}...", project.config.name);
        let repo = self.open_stash()?;

        let id = crate::repo::snapshot(repo, project, true)?;

        info!("Stashed with snapshot id {}", id);
        info!(
            "To restore, run `sprout un-stash` or `sprout un-stash {}`",
            id
        );

        Ok(())
    }

    pub fn restore(&self, project: &Project, snap_id: Id) -> anyhow::Result<()> {
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
