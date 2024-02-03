use crate::wordpress::WordPress;
use rustic_backend::BackendOptions;
use rustic_core::{
    BackupOptions, ConfigOptions, KeyOptions, PathList, Repository, RepositoryOptions,
    SnapshotOptions,
};
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::{error::Error, path::PathBuf};
use tempfile::tempdir;

pub fn initialise() -> Result<(), Box<dyn Error>> {
    // Display info logs
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());

    // Initialize Backends
    let backends = BackendOptions::default()
        .repository("/tmp/repo")
        .to_backends()?;

    // Init repository
    let repo_opts = RepositoryOptions::default().password("test");
    let key_opts = KeyOptions::default();
    let config_opts = ConfigOptions::default();
    let _repo = Repository::new(&repo_opts, backends)?.init(&key_opts, &config_opts)?;

    // -> use _repo for any operation on an open repository
    Ok(())
}

pub fn snapshot(wp: &WordPress, branch: String) -> anyhow::Result<()> {
    // Display info logs
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());

    // Initialize Backends
    let backends = BackendOptions::default()
        .repository("/tmp/repo")
        .to_backends()?;

    // Open repository
    let repo_opts = RepositoryOptions::default().password("test");

    let repo = Repository::new(&repo_opts, backends)?
        .open()?
        .to_indexed_ids()?;

    let backup_opts =
        BackupOptions::default().as_path(PathBuf::from(format!("/.sprout/{}/uploads", branch)));
    let source = PathList::from_string(&wp.get_uploads_dir()?)?;
    let snap = SnapshotOptions::default()
        .add_tags("uploads")?
        .host(wp.get_project_identifier()?)
        .to_snapshot()?;

    // Create snapshot
    let snap = repo.backup(&backup_opts, &source, snap)?;

    println!("successfully created uploads snapshot:\n{snap:#?}");

    let dir = tempdir()?;
    let db_filename = dir.path().join("database.sql");

    wp.dump_database(&db_filename)?;

    let backup_opts =
        BackupOptions::default().as_path(PathBuf::from(format!("/.sprout/{}/database", branch)));

    let source = PathList::from_string(&db_filename.to_string_lossy())?;

    let snap = SnapshotOptions::default()
        .add_tags("database")?
        .host(wp.get_project_identifier()?)
        .to_snapshot()?;

    // Create snapshot
    let snap = repo.backup(&backup_opts, &source, snap)?;

    println!("successfully created db snapshot:\n{snap:#?}");

    Ok(())
}
