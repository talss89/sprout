use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use pretty_assertions::assert_eq;
use sprout::engine::Engine;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

pub mod fixture_facts;

pub type TestResult = Result<()>;

#[derive(Debug)]
pub struct TestContext {
    pub sprout_home: TempDir,
    pub stash_path: TempDir,
    pub repo_path: TempDir,
    pub engine: Engine,
}

impl TestContext {
    pub fn new() -> Result<Self> {
        let sprout_home = TempDir::new()?;
        let stash_path = TempDir::new()?;
        let repo_path = TempDir::new()?;

        Ok(Self {
            engine: Engine {
                sprout_home: sprout_home.path().to_path_buf(),
            },
            sprout_home,
            stash_path,
            repo_path,
        })
    }
}

pub fn run(args: &[&str], expected_file: &str) -> Result<()> {
    let expected = fs::read_to_string(expected_file)?;
    let output = Command::cargo_bin("sprout")?
        .args(args)
        .output()
        .expect("fail");

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    assert_eq!(stdout, expected);

    Ok(())
}
