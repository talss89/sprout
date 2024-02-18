mod common;

use crate::common::TestResult;
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_prints_usage() -> TestResult {
    Command::cargo_bin("sprout")?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
    Ok(())
}
