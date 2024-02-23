mod common;

use std::{fs, path::Path};

use crate::common::{content_generator, TestProjectContext, TestResult};
use assert_cmd::Command;
use common::TestContext;
use predicates::prelude::*;

use rustic_backend::BackendOptions;
use sprout::{project::Project, repo::definition::RepositoryDefinition, stash::Stash};

#[test]
fn test_prints_usage() -> TestResult {
    Command::cargo_bin("sprout")?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
    Ok(())
}

#[test]
fn test_creates_sprout_home() -> TestResult {
    let ctx = TestContext::new()?;

    ctx.engine.ensure_home()?;

    assert!(
        ctx.engine.sprout_home.join("sprout-config.yaml").exists(),
        "sprout-config.yaml was not created"
    );

    assert!(
        ctx.engine.sprout_home.join("repos").exists(),
        "sprout home ./repos was not created"
    );

    let sprout_config = ctx.engine.get_config();

    assert!(
        sprout_config.is_ok(),
        "Could not deserialise default sprout-config.yaml"
    );

    Ok(())
}

#[test]
fn test_creates_stash() -> TestResult {
    let ctx = TestContext::new()?;

    ctx.engine.ensure_home()?;

    Stash::new(&ctx.engine, ctx.engine.get_stash_path())?;

    assert!(
        ctx.engine.get_stash_path().join("config").exists(),
        "Stash was not created"
    );

    Ok(())
}

#[test]
fn test_repo_definitions() -> TestResult {
    let ctx = TestContext::new()?;

    ctx.engine.ensure_home()?;

    RepositoryDefinition::create(
        &RepositoryDefinition {
            repo_key: "TEST1".to_string(),
            repo: BackendOptions::default(),
        },
        &ctx.engine.get_home().join("repos/test-1.yaml"),
    )?;

    RepositoryDefinition::create(
        &RepositoryDefinition {
            repo_key: "TEST2".to_string(),
            repo: BackendOptions::default(),
        },
        &ctx.engine.get_home().join("repos/test-2.yaml"),
    )?;

    assert_eq!(
        RepositoryDefinition::list(&ctx.engine)?.len(),
        2,
        "Repository definitions were not saved"
    );

    assert_eq!(
        RepositoryDefinition::get(&ctx.engine, "test-2")?.1.repo_key,
        "TEST2".to_string(),
        "Access key not saved correctly"
    );

    assert!(
        RepositoryDefinition::create(
            &RepositoryDefinition {
                repo_key: "TEST2".to_string(),
                repo: BackendOptions::default(),
            },
            &ctx.engine.get_home().join("repos/test-2.yaml"),
        )
        .is_err(),
        "Duplicate repo definition should generate an error!"
    );

    Ok(())
}

#[test]
fn test_invalid_project() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    ctx.engine.ensure_home()?;

    let project = Project::new(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts,
    );

    assert!(
        project.is_err(),
        "Uninitialised project should return Err()"
    );

    Ok(())
}

#[test]
fn test_project_snapshot() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    let size_limit = 100 * 1024 * 1024;

    ctx.setup_single_repo()?;

    let _ = content_generator::generate_random_uploads(
        &project_ctx
            .project_path
            .path()
            .join("uploads")
            .to_path_buf(),
        size_limit,
    )?;

    let project = Project::initialise(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts,
    )?;

    let repo = project.open_repo("TEST")?;
    let snapshot = repo.snapshot(true, None, None)?;

    assert!(
        snapshot
            .snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_bytes_processed
            < size_limit
            && snapshot
                .snapshot
                .summary
                .as_ref()
                .unwrap()
                .total_bytes_processed
                > size_limit - (5 * 1024 * 1024),
        "Snapshot of uploads should be within 5MB of test size. Saw: {}, expected: >{}",
        snapshot
            .snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_bytes_processed,
        size_limit - (5 * 1024 * 1024)
    );

    let snapshot = repo.snapshot(true, None, None)?;

    let (snapshots, errors) = project.get_all_snapshots(&repo)?;

    assert_eq!(errors.len(), 0, "Listing snapshots returned errors");

    assert_eq!(
        snapshots.len(),
        2,
        "Expected 2 snapshots, got {}",
        snapshots.len(),
    );

    let latest = project.get_active_snapshot(&repo)?;

    assert_eq!(
        latest.id, snapshot.id,
        "Latest snapshot returned from repo was not the last snapshot we took"
    );

    Ok(())
}

#[test]
fn test_snapshot_branching() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    let size_limit = 10 * 1024 * 1024;

    ctx.setup_single_repo()?;

    let _ = content_generator::generate_random_uploads(
        &project_ctx
            .project_path
            .path()
            .join("uploads")
            .to_path_buf(),
        size_limit,
    )?;

    let mut project = Project::initialise(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts,
    )?;

    let repo = project.open_repo("TEST")?;
    let snapshot = repo.snapshot(true, None, None)?;

    assert_eq!(
        snapshot.get_branch()?,
        "main",
        "Snapshot should default to `main` branch"
    );

    project.config.branch = "other-branch".to_string();

    let repo = project.open_repo("TEST")?;
    let new_snapshot = repo.snapshot(false, None, None)?;

    assert_eq!(
        new_snapshot.get_branch()?,
        "other-branch",
        "Snapshot should respect project branch setting"
    );

    project.update_snapshot_id(new_snapshot.id, project.config.branch.to_owned())?;

    let (snapshots, errors) = project.get_all_snapshots(&repo)?;

    assert_eq!(errors.len(), 0, "Listing snapshots returned errors");

    assert_eq!(
        snapshots.len(),
        2,
        "Expected 2 snapshots, got {}",
        snapshots.len(),
    );

    let latest = project.get_active_snapshot(&repo)?;

    assert_eq!(
        latest.get_branch()?,
        "other-branch",
        "Latest snapshot returned from repo was not on the most recent branch"
    );

    Ok(())
}

#[test]
fn test_project_snapshot_respected() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    let size_limit = 100 * 1024 * 1024;

    ctx.setup_single_repo()?;

    let _ = content_generator::generate_random_uploads(
        &project_ctx
            .project_path
            .path()
            .join("uploads")
            .to_path_buf(),
        size_limit,
    )?;

    let mut project = Project::initialise(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts.clone(),
    )?;

    let repo = project.open_repo("TEST")?;

    let snapshot = repo.snapshot(true, None, None)?;
    let snapshot_2 = repo.snapshot(true, None, None)?;
    let snapshot_3 = repo.snapshot(true, None, None)?;

    let (snapshots, errors) = project.get_all_snapshots(&repo)?;

    assert_eq!(errors.len(), 0, "Listing snapshots returned errors");

    assert_eq!(
        snapshots.len(),
        3,
        "Expected 3 snapshots, got {}",
        snapshots.len(),
    );

    let latest = project.get_active_snapshot(&repo)?;

    assert_eq!(
        latest.id, snapshot_3.id,
        "Latest snapshot returned from repo was not the last snapshot we took"
    );

    project.update_snapshot_id(snapshot_2.id, snapshot.get_branch()?)?;

    let project = Project::new(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts,
    )?;

    assert_eq!(
        project.config.snapshot.unwrap(),
        snapshot_2.id,
        "Updated snapshot was not saved against project sprout.yaml"
    );

    let repo = project.open_repo("TEST")?;
    let current = project.get_active_snapshot(&repo)?;

    assert_eq!(
        current.id, snapshot_2.id,
        "Project reported the wrong current snapshot ID"
    );

    Ok(())
}

#[test]
fn test_seeding() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    ctx.setup_single_repo()?;
    project_ctx.apply_fixture("01_upload_diff_a")?;

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .is_file(),
        "Fixture is missing uploads/1.txt"
    );

    let project = Project::initialise(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts.clone(),
    )?;

    let repo = project.open_repo("TEST")?;
    let snapshot = repo.snapshot(true, None, None)?;

    assert_eq!(
        snapshot.get_total_files(),
        4, // 3 (fixture) + 1 (db)
        "Snapshot has wrong fixture file count"
    );

    project_ctx.wipe_uploads()?;

    assert!(
        !Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .exists(),
        "Uploads not wiped"
    );

    let active = project.get_active_snapshot(&repo)?;

    project.restore_from_snapshot(&repo, &active)?;

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .exists(),
        "1.txt not restored from snapshot"
    );

    Ok(())
}

#[test]
fn test_seeding_intersect() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    ctx.setup_single_repo()?;
    project_ctx.apply_fixture("01_upload_diff_a")?;

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .is_file(),
        "Fixture is missing uploads/1.txt"
    );

    let project = Project::initialise(
        &ctx.engine,
        project_ctx.project_path.path().to_path_buf(),
        project_ctx.facts.clone(),
    )?;

    let repo = project.open_repo("TEST")?;
    let snapshot_a = repo.snapshot(true, None, None)?;

    assert_eq!(
        snapshot_a.get_total_files(),
        4, // 3 (fixture) + 1 (db)
        "Snapshot has wrong fixture file count"
    );

    project_ctx.wipe_uploads()?;
    project_ctx.apply_fixture("02_upload_diff_b")?;

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("4.txt")
            .is_file(),
        "Fixture is missing uploads/4.txt"
    );

    let snapshot_b = repo.snapshot(true, None, None)?;

    assert_eq!(
        snapshot_b.get_total_files(),
        4, // 3 (fixture) + 1 (db)
        "Snapshot has wrong fixture file count"
    );

    project_ctx.wipe_uploads()?;

    assert!(
        !Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .exists(),
        "Uploads not wiped"
    );

    let active = project.get_active_snapshot(&repo)?;

    project.restore_from_snapshot(&repo, &active)?;

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("4.txt")
            .exists(),
        "4.txt not restored from snapshot"
    );

    project.restore_from_snapshot(&repo, &snapshot_a)?;

    assert!(
        !Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("4.txt")
            .exists(),
        "4.txt should not exist"
    );

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("1.txt")
            .exists(),
        "1.txt should exist"
    );

    assert!(
        Path::new(&project_ctx.facts.get_uploads_dir()?)
            .join("3.txt")
            .exists()
            && fs::read_to_string(Path::new(&project_ctx.facts.get_uploads_dir()?).join("3.txt"))?
                == "Three A",
        "3.txt should exist, but be version A"
    );

    Ok(())
}

#[test]
fn test_project_path_escape_safety() -> TestResult {
    let ctx = TestContext::new()?;
    let project_ctx = TestProjectContext::new("https://invalid-project.test")?;

    ctx.setup_single_repo()?;
    project_ctx.apply_fixture("03_unsafe_uploads_path")?;

    assert!(
        Project::new(
            &ctx.engine,
            project_ctx.project_path.path().to_path_buf(),
            project_ctx.facts.clone()
        )
        .is_err(),
        "Uploads path traversal ../ should result in error"
    );

    project_ctx.apply_fixture("04_unsafe_uploads_path")?;

    assert!(
        Project::new(
            &ctx.engine,
            project_ctx.project_path.path().to_path_buf(),
            project_ctx.facts.clone()
        )
        .is_err(),
        "Uploads path traversal ../../ should result in error"
    );

    project_ctx.apply_fixture("05_unsafe_uploads_path")?;

    assert!(
        Project::new(
            &ctx.engine,
            project_ctx.project_path.path().to_path_buf(),
            project_ctx.facts.clone()
        )
        .is_err(),
        "Absolute uploads path should result in error"
    );

    Ok(())
}
