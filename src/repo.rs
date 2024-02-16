use crate::{progress::SproutProgressBar, project::Project, snapshot::Snapshot, PKG_VERSION};

use log::{info, warn};
use rustic_backend::BackendOptions;
use rustic_core::{
    repofile::{Node, SnapshotFile},
    BackupOptions, ConfigOptions, Id, KeyOptions, OpenStatus, ParentOptions, PathList,
    RepositoryOptions, SnapshotOptions,
};

use std::{fs, path::PathBuf};
use tempfile::tempdir;

pub mod definition;
use std::collections::HashMap;

pub type RusticRepo<O> = rustic_core::Repository<SproutProgressBar, O>;

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
        let repo = rustic_core::Repository::new_with_progress(
            &repo_opts,
            backend.to_backends()?,
            SproutProgressBar {},
        )?;

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

        let mut backup_opts =
            BackupOptions::default().as_path(PathBuf::from("/.sprout/database/database.sql"));

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

        let snap = SnapshotOptions::default()
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
            .command(format!("sprout-{}", PKG_VERSION))
            .to_snapshot()?;

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
        let mut backup_opts = BackupOptions::default().as_path(PathBuf::from("/.sprout/uploads"));

        if !automatic_parent {
            if let Some(parent_id) = self.project.config.snapshot {
                if let Ok(parent_snapshot) = Snapshot::from_db_snapshot_id(repo, parent_id) {
                    backup_opts = backup_opts.parent_opts(ParentOptions::default().parent(Some(
                        parent_snapshot.uploads_snapshot.id.to_hex().to_string(),
                    )));
                } else {
                    warn!("The snapshot ID in your `sprout.yaml` file does not exist in this repo. Using an automatic parent instead.");
                }
            }
        }

        let repo = repo.clone().open()?.to_indexed_ids()?;

        let source = PathList::from_string(
            &fs::canonicalize(&self.project.config.uploads_path)
                .unwrap()
                .to_string_lossy(),
        )?;
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

        snap.program_version = format!("sprout-{}", PKG_VERSION);

        // Create snapshot
        Ok(repo.backup(&backup_opts, &source, snap)?)
    }

    pub fn snapshot(&self, automatic_parent: bool) -> anyhow::Result<Snapshot> {
        let db_snapshot = self.snapshot_db(&self.repo, automatic_parent)?;
        let uploads_snapshot =
            self.snapshot_uploads(&self.repo, db_snapshot.id, automatic_parent)?;

        Ok(Snapshot {
            id: db_snapshot.id,
            db_snapshot,
            uploads_snapshot,
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
                    && snap.tags.contains("sprt_obj:database")
                {
                    return true;
                }

                false
            })?;

        Snapshot::from_db_snapshot(&self.repo, &db_snapshot)
    }

    pub fn get_latest_snapshot_for_branch(
        &self,
        project: &Project,
        branch: &str,
    ) -> anyhow::Result<Snapshot> {
        let db_snapshot = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == project.config.name
                    && snap.tags.contains("sprt_obj:database")
                    && snap.tags.contains(&format!("sprt_branch:{}", branch))
                {
                    return true;
                }

                false
            })?;

        Snapshot::from_db_snapshot(&self.repo, &db_snapshot)
    }

    pub fn get_all_snapshots_for_project(
        &self,
        project: &Project,
    ) -> anyhow::Result<(Vec<Snapshot>, Vec<anyhow::Error>)> {
        let db_snapshots = self
            .repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_matching_snapshots(|snap| {
                if snap.hostname == project.config.name {
                    return true;
                }

                false
            })?;

        let mut db_snapshot_map: HashMap<String, SnapshotFile> = HashMap::new();
        let mut uploads_snapshot_map: HashMap<String, SnapshotFile> = HashMap::new();
        let mut errors = vec![];

        for snap in db_snapshots.iter() {
            if snap.tags.contains("sprt_obj:database") {
                db_snapshot_map.insert(snap.id.to_hex().to_string(), snap.clone());
            } else if snap.tags.contains("sprt_obj:uploads") {
                if let Ok(id) = Snapshot::get_sprout_tag(snap, "sprt_db") {
                    uploads_snapshot_map.insert(id, snap.clone());
                }
            }
        }

        let mut snapshots: Vec<Snapshot> = db_snapshot_map
            .into_iter()
            .filter_map(|(id_string, snap)| {
                if uploads_snapshot_map.contains_key(&id_string) {
                    return match Id::from_hex(&id_string) {
                        Ok(id) => Some(Snapshot {
                            id,
                            uploads_snapshot: uploads_snapshot_map.get(&id_string).unwrap().clone(),
                            db_snapshot: snap.clone(),
                        }),
                        Err(e) => {
                            errors.push(anyhow::anyhow!(e));
                            None
                        }
                    };
                } else {
                    errors.push(anyhow::anyhow!(
                        "Missing uploads snapshot for {}",
                        &id_string
                    ));
                    None
                }
            })
            .collect();

        snapshots.sort_by(|a, b| b.db_snapshot.time.cmp(&a.db_snapshot.time));

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
                    && snap.tags.contains("sprt_obj:database")
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

        Ok(
            repo.node_from_snapshot_path("latest:/.sprout/uploads", |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:uploads")
                    && snap
                        .tags
                        .contains(&format!("sprt_db:{}", snapshot.id.to_hex().as_str()))
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.project.config.branch))
                {
                    return true;
                }

                false
            })?,
        )
    }

    pub fn get_db_node(&self, snapshot: &Snapshot) -> anyhow::Result<Node> {
        let repo = self.repo.clone().open()?.to_indexed()?;

        Ok(
            repo.node_from_snapshot_path(&format!("{}:/.sprout/database", snapshot.id), |snap| {
                if snap.hostname == self.project.config.name
                    && snap.tags.contains("sprt_obj:database")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.project.config.branch))
                {
                    return true;
                }

                false
            })?,
        )
    }
}

// pub fn restore(repository: RusticRepo<()>, project: &Project, snap_id: Id) -> anyhow::Result<()> {
//     let repo = repository.open()?.to_indexed()?;

//     let ident = project.config.name.to_owned();

//     let node = repo.node_from_snapshot_path(&format!("latest:/.sprout/uploads"), |snap| {
//         if snap.hostname == ident
//             && snap.tags.contains("sprt_obj:uploads")
//             && snap
//                 .tags
//                 .contains(&format!("sprt_db:{}", snap_id.to_hex().as_str()))
//             && snap
//                 .tags
//                 .contains(&format!("sprt_branch:{}", project.config.branch))
//         {
//             return true;
//         }

//         false
//     })?;

//     // use list of the snapshot contents using no additional filtering
//     let streamer_opts = LsOptions::default();
//     let ls = repo.ls(&node, &streamer_opts)?;

//     let destination = fs::canonicalize(project.config.uploads_path.to_owned())?; // restore to this destination dir
//     let create = true; // create destination dir, if it doesn't exist
//     let dest = LocalDestination::new(&destination.to_string_lossy(), create, !node.is_dir())?;

//     let opts = RestoreOptions::default();
//     let dry_run = false;
//     // create restore infos. Note: this also already creates needed dirs in the destination
//     let restore_infos = repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

//     repo.restore(restore_infos, &opts, ls, &dest)?;

//     let dir = tempdir()?;

//     let node = repo.node_from_snapshot_path(&format!("{}:/.sprout/database", snap_id), |snap| {
//         if snap.hostname == ident
//             && snap.tags.contains("sprt_obj:database")
//             && snap
//                 .tags
//                 .contains(&format!("sprt_branch:{}", project.config.branch))
//         {
//             return true;
//         }

//         false
//     })?;

//     // use list of the snapshot contents using no additional filtering
//     let streamer_opts = LsOptions::default();
//     let ls = repo.ls(&node, &streamer_opts)?;

//     let destination = dir.path(); // restore to this destination dir
//     let create = true; // create destination dir, if it doesn't exist
//     let dest = LocalDestination::new(&destination.to_string_lossy(), create, !node.is_dir())?;

//     let opts = RestoreOptions::default();
//     let dry_run = false;
//     // create restore infos. Note: this also already creates needed dirs in the destination
//     let restore_infos = repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

//     repo.restore(restore_infos, &opts, ls, &dest)?;

//     project.import_database(destination.join("database.sql"))?;

//     Ok(())
// }
