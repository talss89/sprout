use crate::{progress::SproutProgressBar, project::Project, snapshot::Snapshot, PKG_VERSION};

use log::{info, warn};
use rustic_backend::BackendOptions;
use rustic_core::{
    last_modified_node,
    repofile::{Node, SnapshotFile},
    BackupOptions, ConfigOptions, Id, KeyOptions, LocalSourceSaveOptions, OpenStatus,
    ParentOptions, PathList, RepositoryOptions, SnapshotOptions,
};

use std::{fs, path::PathBuf};
use tempfile::tempdir;

pub mod definition;

pub type RusticRepo<O> = rustic_core::Repository<SproutProgressBar, O>;

pub trait RusticRepoFactory {
    fn open_repo(
        backend: BackendOptions,
        repo_opts: RepositoryOptions,
    ) -> anyhow::Result<RusticRepo<()>> {
        Ok(rustic_core::Repository::new_with_progress(
            &repo_opts,
            backend.to_backends()?,
            SproutProgressBar {},
        )?)
    }
}

impl<O> RusticRepoFactory for RusticRepo<O> {}

pub struct ProjectRepository {
    pub repo: RusticRepo<()>,
    project: Project,
}

impl ProjectRepository {
    pub fn new(
        project: &Project,
        backend: BackendOptions,
        repo_opts: RepositoryOptions,
    ) -> anyhow::Result<Self> {
        let repo = RusticRepo::<()>::open_repo(backend, repo_opts)?;

        Ok(Self {
            repo,
            project: project.clone(),
        })
    }

    /// Initialise a new repo
    pub fn initialise(
        backend: BackendOptions,
        repo_opts: RepositoryOptions,
        key_opts: KeyOptions,
        config_opts: ConfigOptions,
    ) -> anyhow::Result<RusticRepo<OpenStatus>> {
        let repo = rustic_core::Repository::new_with_progress(
            &repo_opts,
            backend.to_backends()?,
            SproutProgressBar {},
        )?;

        Ok(repo.init(&key_opts, &config_opts)?)
    }

    fn snapshot_db(
        &self,
        repo: &RusticRepo<()>,
        automatic_parent: bool,
    ) -> anyhow::Result<SnapshotFile> {
        let repo = repo.clone().open()?.to_indexed_ids()?;
        let dir = tempdir()?;
        let db_filename = dir.path().join("database.sql");

        self.project.dump_database(&db_filename)?;

        let mut backup_opts = BackupOptions::default()
            .as_path(PathBuf::from("/.sprout/database/database.sql"))
            .ignore_save_opts(LocalSourceSaveOptions::default().ignore_devid(true));

        if !automatic_parent {
            backup_opts = backup_opts.parent_opts(
                ParentOptions::default().parent(
                    self.project
                        .config
                        .snapshot
                        .map(|id| id.to_hex().to_string()),
                ),
            );
        }

        let source = PathList::from_string(&db_filename.to_string_lossy())?;

        let mut snap = SnapshotOptions::default()
            .add_tags(
                format!(
                    "sprt_obj:database,sprt_uniq:{},sprt_branch:{}",
                    self.project
                        .unique_hash
                        .as_ref()
                        .unwrap_or(&"_none_".to_string()),
                    self.project.config.branch
                )
                .as_str(),
            )?
            .host(self.project.config.name.to_owned())
            .to_snapshot()?;

        snap.program_version = format!("sprout {}", PKG_VERSION);

        // Create snapshot
        let snap = repo.backup(&backup_opts, &source, snap)?;

        info!("Successfully created DB snapshot");

        Ok(snap)
    }

    fn snapshot_uploads(
        &self,
        repo: &RusticRepo<()>,
        database_snapshot_id: Id,
        automatic_parent: bool,
    ) -> anyhow::Result<SnapshotFile> {
        let mut backup_opts = BackupOptions::default()
            .as_path(PathBuf::from("/.sprout/uploads"))
            .ignore_save_opts(LocalSourceSaveOptions::default().ignore_devid(true));

        if !automatic_parent {
            if let Some(parent_id) = self.project.config.snapshot {
                if let Ok(parent_snapshot) = Snapshot::from_snapshot_id(repo, parent_id) {
                    backup_opts = backup_opts.parent_opts(
                        ParentOptions::default()
                            .parent(Some(parent_snapshot.snapshot.id.to_hex().to_string())),
                    );
                } else {
                    warn!("The snapshot ID in your `sprout.yaml` file does not exist in this repo. Using an automatic parent instead.");
                }
            }
        }

        let repo = repo.clone().open()?.to_indexed_ids()?;

        let resolved_uploads_path =
            fs::canonicalize(&self.project.path)?.join(&self.project.config.uploads_path);

        if !resolved_uploads_path.exists() {
            fs::create_dir_all(&resolved_uploads_path)?;
        }

        let source = PathList::from_string(&resolved_uploads_path.to_string_lossy())?;

        let mut snap = SnapshotOptions::default()
            .add_tags(
                format!(
                    "sprt_obj:uploads,sprt_db:{},sprt_uniq:{},sprt_branch:{}",
                    database_snapshot_id.to_hex().as_str(),
                    self.project
                        .unique_hash
                        .as_ref()
                        .unwrap_or(&"_none_".to_string()),
                    self.project.config.branch
                )
                .as_str(),
            )?
            .host(self.project.config.name.to_owned())
            .to_snapshot()?;

        snap.program_version = format!("sprout {}", PKG_VERSION);

        // Create snapshot
        Ok(repo.backup(&backup_opts, &source, snap)?)
    }

    pub fn snapshot(&self, automatic_parent: bool) -> anyhow::Result<Snapshot> {
        let db_snapshot = self.snapshot_db(&self.repo, automatic_parent)?;
        let uploads_snapshot =
            self.snapshot_uploads(&self.repo, db_snapshot.id, automatic_parent)?;

        let mut merged = SnapshotOptions::default()
            .add_tags(
                format!(
                    "sprt_obj:bundle,sprt_uniq:{},sprt_branch:{},sprt_stats:{}",
                    self.project
                        .unique_hash
                        .as_ref()
                        .unwrap_or(&"_none_".to_string()),
                    self.project.config.branch,
                    Snapshot::pack_stats(&db_snapshot, &uploads_snapshot)?
                )
                .as_str(),
            )?
            .host(self.project.config.name.to_owned())
            .to_snapshot()?;

        let snapshots = &[db_snapshot, uploads_snapshot];

        merged.program_version = format!("sprout {}", PKG_VERSION);

        let repo = self.repo.clone().open()?.to_indexed_ids()?;

        let merged = repo.merge_snapshots(snapshots, &last_modified_node, merged)?;

        let snap_ids: Vec<_> = snapshots.iter().map(|sn| sn.id).collect();
        repo.delete_snapshots(&snap_ids)?;

        Ok(Snapshot {
            id: merged.id,
            snapshot: merged,
        })
    }

    pub fn get_latest_snapshot(&self) -> anyhow::Result<Snapshot> {
        let db_snapshot = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:bundle")
                {
                    return true;
                }

                false
            })?;

        Snapshot::from_snapshot(&db_snapshot)
    }

    pub fn get_latest_snapshot_for_branch(
        &self,
        project: &Project,
        branch: &str,
    ) -> anyhow::Result<Snapshot> {
        let snapshot = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == project.config.name
                    && snap.tags.contains("sprt_obj:bundle")
                    && snap.tags.contains(&format!("sprt_branch:{}", branch))
                {
                    return true;
                }

                false
            })?;

        Snapshot::from_snapshot(&snapshot)
    }

    pub fn get_all_snapshots_for_project(
        &self,
        project: &Project,
    ) -> anyhow::Result<(Vec<Snapshot>, Vec<anyhow::Error>)> {
        let snapshots = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_matching_snapshots(|snap| {
                if snap.hostname == project.config.name && snap.tags.contains("sprt_obj:bundle") {
                    return true;
                }

                false
            })?;

        let errors = vec![];

        let mut snapshots: Vec<Snapshot> = snapshots
            .into_iter()
            .map(|snap| Snapshot::from_snapshot(&snap).unwrap())
            .collect();

        snapshots.sort_by(|a, b| b.snapshot.time.cmp(&a.snapshot.time));

        Ok((snapshots, errors))
    }

    pub fn get_latest_unique_hash(&self) -> anyhow::Result<Option<String>> {
        let node = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:bundle")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.project.config.branch))
                {
                    return true;
                }

                false
            });

        match node {
            Err(_) => Ok(None),

            Ok(file) => Ok(file
                .tags
                .iter()
                .filter(|e| e.starts_with("sprt_uniq:"))
                .map(|e| e.replace("sprt_uniq:", ""))
                .collect::<Vec<String>>()
                .first()
                .cloned()),
        }
    }

    pub fn get_uploads_node(&self, snapshot: &Snapshot) -> anyhow::Result<Node> {
        let repo = self.repo.clone().open()?.to_indexed()?;

        Ok(repo.node_from_snapshot_path(
            &format!("{}:/.sprout/uploads", snapshot.id.to_hex().as_str()),
            |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:bundle")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.project.config.branch))
                {
                    return true;
                }

                false
            },
        )?)
    }

    pub fn get_db_node(&self, snapshot: &Snapshot) -> anyhow::Result<Node> {
        let repo = self.repo.clone().open()?.to_indexed()?;

        Ok(repo.node_from_snapshot_path(
            &format!("{}:/.sprout/database", snapshot.id.to_hex().as_str()),
            |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:bundle")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.project.config.branch))
                {
                    return true;
                }

                false
            },
        )?)
    }
}
