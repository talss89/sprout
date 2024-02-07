use crate::project::Project;
use capturing_glob::glob;
use duration_macro::duration;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use rustic_backend::BackendOptions;
use rustic_core::{
    BackupOptions, ConfigOptions, Id, KeyOptions, LocalDestination, LsOptions, PathList, Progress,
    ProgressBars, Repository, RepositoryOptions, RestoreOptions, SnapshotOptions,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fs, path::PathBuf};
use tempfile::tempdir;

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryDefinition {
    pub access_key: String,

    #[serde(flatten)]
    pub repo: BackendOptions,
}

pub struct Repositories {}

impl Repositories {
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

pub fn open_repo(
    backend: &BackendOptions,
    repo_opts: RepositoryOptions,
) -> anyhow::Result<Repository<SproutProgressBar, ()>> {
    // Initialize Backends
    let backends = backend.to_backends()?;

    // Init repository
    let repo = Repository::new_with_progress(&repo_opts, backends, SproutProgressBar {})?;
    Ok(repo)
}

pub fn initialise(repo: Repository<SproutProgressBar, ()>) -> anyhow::Result<()> {
    let key_opts = KeyOptions::default();
    let config_opts = ConfigOptions::default();
    let _repo = repo.init(&key_opts, &config_opts)?;

    // -> use _repo for any operation on an open repository
    Ok(())
}

pub fn snapshot(
    repository: Repository<SproutProgressBar, ()>,
    project: &Project,
) -> anyhow::Result<Id> {
    let repo = repository.open()?.to_indexed_ids()?;

    let dir = tempdir()?;
    let db_filename = dir.path().join("database.sql");

    project.dump_database(&db_filename)?;

    let backup_opts = BackupOptions::default().as_path(PathBuf::from(format!(
        "/.sprout/{}/database/database.sql",
        project.config.branch
    )));

    let source = PathList::from_string(&db_filename.to_string_lossy())?;

    let snap = SnapshotOptions::default()
        .add_tags(
            format!(
                "sprt_obj:database,sprt_uniq:{},sprt_branch:{}",
                project
                    .unique_hash
                    .as_ref()
                    .unwrap_or(&"_none_".to_string()),
                project.config.branch
            )
            .as_str(),
        )?
        .host(project.config.name.to_owned())
        .to_snapshot()?;

    // Create snapshot
    let snap = repo.backup(&backup_opts, &source, snap)?;

    info!("Successfully created DB snapshot");

    let database_snap_id = snap.id;

    let backup_opts = BackupOptions::default().as_path(PathBuf::from(format!(
        "/.sprout/{}/uploads",
        project.config.branch
    )));
    let source = PathList::from_string(
        &fs::canonicalize(&project.config.uploads_path)
            .unwrap()
            .to_string_lossy(),
    )?;
    let snap = SnapshotOptions::default()
        .add_tags(
            format!(
                "sprt_obj:uploads,sprt_db:{},sprt_uniq:{},sprt_branch:{}",
                database_snap_id,
                project
                    .unique_hash
                    .as_ref()
                    .unwrap_or(&"_none_".to_string()),
                project.config.branch
            )
            .as_str(),
        )?
        .host(project.config.name.to_owned())
        .to_snapshot()?;

    // Create snapshot
    let _snap = repo.backup(&backup_opts, &source, snap)?;

    info!("Successfully created uploads snapshot");

    Ok(database_snap_id)
}

pub fn restore(
    repository: Repository<SproutProgressBar, ()>,
    project: &Project,
    snap_id: String,
) -> anyhow::Result<()> {
    let repo = repository.open()?.to_indexed()?;

    let ident = project.config.name.to_owned();

    let node = repo.node_from_snapshot_path(
        &format!("latest:/.sprout/{}/uploads", project.config.branch),
        |snap| {
            if snap.hostname == ident
                && snap.tags.contains("sprt_obj:uploads")
                && snap.tags.contains(&format!("sprt_db:{}", snap_id))
            {
                return true;
            }

            false
        },
    )?;

    // use list of the snapshot contents using no additional filtering
    let streamer_opts = LsOptions::default();
    let ls = repo.ls(&node, &streamer_opts)?;

    let destination = fs::canonicalize(project.config.uploads_path.to_owned())?; // restore to this destination dir
    let create = true; // create destination dir, if it doesn't exist
    let dest = LocalDestination::new(&destination.to_string_lossy(), create, !node.is_dir())?;

    let opts = RestoreOptions::default();
    let dry_run = false;
    // create restore infos. Note: this also already creates needed dirs in the destination
    let restore_infos = repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

    repo.restore(restore_infos, &opts, ls, &dest)?;

    let dir = tempdir()?;

    let node = repo.node_from_snapshot_path(
        &format!("{}:/.sprout/{}/database", snap_id, project.config.branch),
        |snap| {
            if snap.hostname == ident && snap.tags.contains("sprt_obj:database") {
                return true;
            }

            false
        },
    )?;

    // use list of the snapshot contents using no additional filtering
    let streamer_opts = LsOptions::default();
    let ls = repo.ls(&node, &streamer_opts)?;

    let destination = dir.path(); // restore to this destination dir
    let create = true; // create destination dir, if it doesn't exist
    let dest = LocalDestination::new(&destination.to_string_lossy(), create, !node.is_dir())?;

    let opts = RestoreOptions::default();
    let dry_run = false;
    // create restore infos. Note: this also already creates needed dirs in the destination
    let restore_infos = repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

    repo.restore(restore_infos, &opts, ls, &dest)?;

    project.import_database(destination.join("database.sql"))?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct SproutProgressBar {}
#[derive(Clone, Debug)]
pub struct SproutProgress {
    pub bar: ProgressBar,
}

impl SproutProgress {
    pub fn new() -> Self {
        Self {
            bar: ProgressBar::new(1024),
        }
    }

    pub fn hidden() -> Self {
        Self {
            bar: ProgressBar::hidden(),
        }
    }

    pub fn spinner() -> Self {
        Self {
            bar: ProgressBar::new_spinner(),
        }
    }
}

impl ProgressBars for SproutProgressBar {
    type P = SproutProgress;

    fn progress_hidden(&self) -> Self::P {
        let p = SproutProgress::hidden();

        p
    }

    fn progress_spinner(&self, prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::spinner();

        p.bar.set_message(prefix);

        p.bar
            .set_style(ProgressStyle::with_template("{spinner:^9.green} {msg}").unwrap());

        p.bar.enable_steady_tick(duration!(100 ms));

        p
    }

    fn progress_counter(&self, _prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::new();
        p.bar.set_style(
            ProgressStyle::with_template(
                "{spinner:^9.green} [{elapsed_precise:.dimmed}] [{wide_bar:.dimmed.cyan/blue}] {pos:.dimmed}/{len:.dimmed} ({eta:.dimmed})",
            )
            .unwrap()
            .progress_chars("#>-"),
        );

        p
    }

    fn progress_bytes(&self, _prefix: impl Into<Cow<'static, str>>) -> Self::P {
        let p = SproutProgress::new();
        p.bar.set_style(ProgressStyle::with_template("{spinner:^9.green} [{elapsed_precise:.dimmed}] [{wide_bar:.dimmed.cyan/blue}] {bytes:.dimmed}/{total_bytes:.dimmed} ({eta:.dimmed})")
        .unwrap()
        .progress_chars("#>-"));

        p
    }
}

impl Progress for SproutProgress {
    fn is_hidden(&self) -> bool {
        self.bar.is_hidden()
    }
    fn set_length(&self, len: u64) {
        self.bar.set_length(len)
    }
    fn set_title(&self, title: &'static str) {
        self.bar.set_message(title);
    }
    fn inc(&self, inc: u64) {
        self.bar.inc(inc);
    }
    fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
