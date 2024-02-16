use std::path::PathBuf;

use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{repofile::SnapshotFile, ConfigOptions, Id, KeyOptions, RepositoryOptions};

use crate::{engine::*, project::Project, repo::ProjectRepository, snapshot::Snapshot};

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

        let key_opts = KeyOptions::default();
        let config_opts = ConfigOptions::default();

        let _repo = ProjectRepository::initialise(backend, repo_opts, key_opts, config_opts);

        let mut sprout_config = get_sprout_config()?;

        sprout_config.stash_key = passkey;

        write_sprout_config(&sprout_config)?;

        Ok(())
    }

    fn open_stash(&self, project: &Project) -> anyhow::Result<ProjectRepository> {
        let sprout_config = get_sprout_config()?;
        let backend =
            BackendOptions::default().repository(self.path.join("stash").to_string_lossy());
        let repo_opts = RepositoryOptions::default().password(&sprout_config.stash_key);

        Ok(ProjectRepository::new(project, backend, repo_opts)?)
    }

    pub fn stash(&self, project: &Project) -> anyhow::Result<()> {
        info!("Stashing {}...", project.config.name);
        let repo = self.open_stash(project)?;

        let snapshot = repo.snapshot(true)?;

        info!("Stashed with snapshot id {}", snapshot.id);
        info!(
            "To restore, run `sprout un-stash` or `sprout un-stash {}`",
            snapshot.id
        );

        Ok(())
    }

    pub fn restore(&self, project: &Project, snap_id: Id) -> anyhow::Result<()> {
        info!("Restoring stash...");
        let repo = self.open_stash(&project)?;
        let snapshot = Snapshot::from_db_snapshot_id(&repo.repo, snap_id)?;

        let _id = project.restore_from_snapshot(&repo, &snapshot)?;

        Ok(())
    }

    pub fn get_latest_stash(&self, project: &Project) -> anyhow::Result<Snapshot> {
        let repo = self.open_stash(project)?;

        let snapshot = repo.get_latest_snapshot(&project)?;

        Ok(snapshot)
    }

    pub fn get_all_stashes_for_project(
        &self,
        project: &Project,
    ) -> anyhow::Result<Vec<(SnapshotFile, Option<SnapshotFile>)>> {
        let repo: ProjectRepository = self.open_stash(project)?;

        // let snapshots = repo
        //     .repo
        //     .open()?
        //     .to_indexed_ids()?
        //     .get_matching_snapshots(|snap| {
        //         if snap.hostname == project.config.name && snap.tags.contains("sprt_obj:database") {
        //             return true;
        //         }

        //         false
        //     })
        //     .unwrap_or(vec![])
        //     .iter()
        //     .map(|f| {
        //         let uploads =
        //             crate::repo::get_uploads_snapshot_by_db_snapshot_id((&repo.repo).clone(), f.id);

        //         (
        //             f.to_owned(),
        //             match uploads {
        //                 Err(_) => None,
        //                 Ok(uploads) => Some(uploads),
        //             },
        //         )
        //     })
        //     .collect::<Vec<(SnapshotFile, Option<SnapshotFile>)>>();

        Ok(vec![])
    }
}
