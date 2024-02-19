mod common;

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
    let snapshot = repo.snapshot(true)?;

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

    let snapshot = repo.snapshot(true)?;

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
