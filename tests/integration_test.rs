mod common;

use crate::common::TestResult;
use assert_cmd::Command;
use common::TestContext;
use predicates::prelude::*;

use sprout::stash::Stash;

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
