use std::fs;
use std::path::Path;

use anyhow::Result;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rustic_backend::BackendOptions;
use rustic_core::{ConfigOptions, KeyOptions, RepositoryOptions};
use sprout::engine::Engine;
use sprout::facts::ProjectFactProvider;
use sprout::repo::definition::RepositoryDefinition;
use sprout::repo::ProjectRepository;
use tempfile::TempDir;

use self::fixture_facts::FixtureFacts;

pub mod content_generator;
pub mod fixture_facts;

pub type TestResult = Result<()>;

#[derive(Debug)]
pub struct TestContext {
    pub sprout_home: TempDir,
    pub repo_path: TempDir,
    pub engine: Engine,
}

impl TestContext {
    pub fn new() -> Result<Self> {
        let sprout_home = TempDir::new()?;
        let repo_path = TempDir::new()?;

        Ok(Self {
            engine: Engine {
                sprout_home: sprout_home.path().to_path_buf(),
            },
            sprout_home,
            repo_path,
        })
    }

    pub fn setup_single_repo(&self) -> Result<()> {
        self.engine.ensure_home()?;

        RepositoryDefinition::create(
            &RepositoryDefinition {
                repo_key: "TEST".to_string(),
                repo: BackendOptions {
                    repository: Some(self.repo_path.path().to_string_lossy().to_string()),
                    ..BackendOptions::default()
                },
            },
            &self.engine.get_home().join("repos/test.yaml"),
        )?;

        let mut sprout_config = self.engine.get_config()?;
        sprout_config.default_repo = "test".to_string();
        self.engine.write_config(&sprout_config)?;

        let (_, definition) = RepositoryDefinition::get(&self.engine, "test")?;
        let repo_opts = RepositoryOptions::default().password(definition.repo_key);

        let _ = ProjectRepository::initialise(
            definition.repo.clone(),
            repo_opts,
            KeyOptions::default(),
            ConfigOptions::default(),
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct TestProjectContext {
    pub project_path: TempDir,
    pub facts: Box<dyn ProjectFactProvider>,
}

impl TestProjectContext {
    pub fn new(home_url: &str) -> Result<Self> {
        let project_path = TempDir::new()?;

        Ok(Self {
            facts: Box::new(FixtureFacts {
                path: project_path.path().to_path_buf(),
                is_wordpress_installed: true,
                home_url: home_url.to_string(),
                hash: Some(
                    thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(32)
                        .map(|x| x as char)
                        .collect(),
                ),
            }),
            project_path,
        })
    }

    pub fn apply_fixture(&self, fixture: &str) -> Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);

        if !path.exists() {
            return Err(anyhow::anyhow!("Fixture {} does not exist", fixture));
        }

        dircpy::CopyBuilder::new(path, &self.project_path)
            .overwrite(true)
            .run()?;

        Ok(())
    }

    pub fn wipe_uploads(&self) -> Result<()> {
        let path = self.facts.get_uploads_dir().unwrap();
        let path = Path::new(&path);

        if path.exists() {
            fs::remove_dir_all(path)?;
        }

        fs::create_dir_all(path)?;

        Ok(())
    }
}
