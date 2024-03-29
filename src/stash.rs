use std::path::PathBuf;

use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{ConfigOptions, Id, KeyOptions, RepositoryOptions};

use crate::{
    engine::*,
    project::Project,
    repo::{ProjectRepository, RusticRepo, RusticRepoFactory},
    snapshot::Snapshot,
};

pub struct Stash {
    pub path: PathBuf,
    engine: Engine,
}

impl Stash {
    pub fn new(engine: &Engine, path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            warn!("Stash does not exist at {}.", path.to_string_lossy());

            Stash::initialise(engine, path.to_owned())?;
        }

        Ok(Self {
            path,
            engine: engine.clone(),
        })
    }

    pub fn initialise(engine: &Engine, path: PathBuf) -> anyhow::Result<()> {
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

        let backend = BackendOptions::default().repository(path.to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(&passkey);

        let key_opts = KeyOptions::default();
        let config_opts = ConfigOptions::default();

        let _repo = ProjectRepository::initialise(backend, repo_opts, key_opts, config_opts);

        let mut sprout_config = engine.get_config()?;

        sprout_config.stash_key = passkey;

        engine.write_config(&sprout_config)?;

        Ok(())
    }

    fn open_stash(&self, project: &Project) -> anyhow::Result<ProjectRepository> {
        let sprout_config = self.engine.get_config()?;
        let backend = BackendOptions::default().repository(self.path.to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(sprout_config.stash_key);

        ProjectRepository::new(project, backend, repo_opts)
    }

    fn direct_open_stash(&self) -> anyhow::Result<RusticRepo<()>> {
        let sprout_config = self.engine.get_config()?;
        let backend = BackendOptions::default().repository(self.path.to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(sprout_config.stash_key);

        RusticRepo::<()>::open_repo(backend, repo_opts)
    }

    pub fn stash(&self, project: &Project) -> anyhow::Result<()> {
        info!("Stashing {}...", project.config.name);
        let repo = self.open_stash(project)?;

        let snapshot = repo.snapshot(true)?;

        info!(
            "Stashed with snapshot id {}",
            snapshot.id.to_hex().to_string()
        );
        info!(
            "To restore, run `sprout un-stash` or `sprout un-stash {}`",
            snapshot.id.to_hex().to_string()
        );

        Ok(())
    }

    pub fn restore(&self, project: &Project, snap_id: Id) -> anyhow::Result<()> {
        info!("Restoring stash...");
        let repo = self.open_stash(project)?;
        let snapshot = Snapshot::from_snapshot_id(&repo.repo, snap_id)?;

        project.restore_from_snapshot(&repo, &snapshot)?;

        Ok(())
    }

    pub fn get_latest_stash(&self, project: &Project) -> anyhow::Result<Snapshot> {
        let repo = self.open_stash(project)?;

        let snapshot = repo.get_latest_snapshot()?;

        Ok(snapshot)
    }

    pub fn get_stash_by_id(&self, id: Id) -> anyhow::Result<Snapshot> {
        let repo = self.direct_open_stash()?;

        let snapshot = Snapshot::from_snapshot_id(&repo, id)?;

        Ok(snapshot)
    }

    pub fn drop(&self, id: Id) -> anyhow::Result<()> {
        let repo = self.direct_open_stash()?;

        let snapshot = Snapshot::from_snapshot_id(&repo, id)?;

        let repo = repo.open()?;

        let ids = vec![snapshot.snapshot.id];

        repo.delete_snapshots(&ids)?;

        Ok(())
    }

    pub fn get_all_stashes_for_project(
        &self,
        project: &Project,
    ) -> anyhow::Result<(Vec<Snapshot>, Vec<anyhow::Error>)> {
        let repo = self.open_stash(project)?;

        repo.get_all_snapshots_for_project(project)
    }
}
