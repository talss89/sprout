use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use capturing_glob::glob;
use dialoguer::Input;

use log::{info, warn};
use rustic_core::{
    Id, LocalDestination, LsOptions, Progress, ProgressBars, RepositoryOptions, RestoreOptions,
};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use crate::{
    engine::Engine,
    facts::ProjectFactProvider,
    progress::SproutProgressBar,
    repo::{definition::RepositoryDefinition, ProjectRepository},
    snapshot::Snapshot,
    theme::CliTheme,
};

use colored::*;

#[derive(Debug, Serialize, Clone)]
pub struct Project {
    pub path: PathBuf,
    pub config: ProjectConfig,
    pub unique_hash: Option<String>,
    pub home_url: String,
    #[serde(skip)]
    facts: Box<dyn ProjectFactProvider>,
    #[serde(skip)]
    engine: Engine,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub branch: String,
    pub snapshot: Option<Id>,
    pub uploads_path: PathBuf,
    pub repo: String,
}

impl Project {
    pub fn new(
        engine: &Engine,
        path: PathBuf,
        facts: Box<dyn ProjectFactProvider>,
    ) -> anyhow::Result<Self> {
        let config = Self::load_project_config(&path.join("sprout.yaml")).map_err(|_| { anyhow::anyhow!("Is this a project? sprout.yaml is missing. Use `sprout init` to initialise a new project.")})?;

        Ok(Self {
            unique_hash: facts.generate_unique_hash()?,
            path,
            home_url: format!("https://{}.test", &config.name),
            config,
            facts,
            engine: engine.clone(),
        })
    }

    pub fn initialise(
        engine: &Engine,
        path: PathBuf,
        facts: Box<dyn ProjectFactProvider>,
    ) -> anyhow::Result<Self> {
        if path.join("sprout.yaml").exists() {
            return Err(anyhow::anyhow!("A sprout.yaml already exists!"));
        }

        let path = fs::canonicalize(path).unwrap();

        let mut uploads_path = PathBuf::from("./wp-content/uploads");
        let sprout_config = engine.get_config()?;

        if let Ok(installed) = facts.is_wordpress_installed() {
            if installed {
                if let Ok(detected_uploads_path) = facts.get_uploads_dir() {
                    uploads_path = PathBuf::from(detected_uploads_path)
                        .strip_prefix(&path)?
                        .to_path_buf();
                }
            }
        }

        let config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            branch: "main".to_string(),
            snapshot: None,
            uploads_path,
            repo: sprout_config.default_repo,
        };

        fs::write(path.join("sprout.yaml"), serde_yaml::to_string(&config)?)?;

        Project::new(engine, path, facts)
    }

    pub fn load_project_config(path: &PathBuf) -> anyhow::Result<ProjectConfig> {
        Ok(serde_yaml::from_slice::<ProjectConfig>(&fs::read(path)?)?)
    }

    pub fn determine_home_url(&mut self) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Loading WordPress project with WP-CLI...");

        let home_url = match self.facts.get_home_url() {
            Ok(url) => url,
            Err(e) => {
                spinner.finish();
                warn!(
                    "Couldn't query wp-cli to determine your current home URL. {}",
                    e
                );
                Input::with_theme(&CliTheme::default())
                    .with_prompt("Please enter your WP_HOME URL.")
                    .default(format!("https://{}.test", &self.config.name))
                    .interact_text()
                    .unwrap()
            }
        };

        spinner.finish();

        self.home_url = home_url;

        Ok(())
    }

    pub fn dump_database(&self, path: &Path) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Exporting database...");
        let ret = self.facts.dump_database(path, &self.home_url);

        spinner.finish();

        ret
    }

    pub fn import_database(&self, path: PathBuf) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Importing database...");

        self.facts.import_database(&path)?;

        spinner
            .bar
            .set_message(format!("Setting home URL to {}", &self.home_url));

        self.facts.postprocess_database(&self.home_url)?;

        spinner.finish();

        info!("Database installed and URL set to {}", &self.home_url);

        Ok(())
    }

    pub fn update_snapshot_id(&mut self, id: Id, branch: String) -> anyhow::Result<()> {
        self.config.snapshot = Some(id);
        self.config.branch = branch;

        fs::write(
            self.path.join("sprout.yaml"),
            serde_yaml::to_string(&self.config)?,
        )?;

        Ok(())
    }

    pub fn open_repo(&self, repo_key: &str) -> anyhow::Result<ProjectRepository> {
        let repo_opts = RepositoryOptions::default().password(repo_key);
        let (_, definition) = RepositoryDefinition::get(&self.engine, self.config.repo.as_str())?;
        let repo = ProjectRepository::new(self, definition.repo, repo_opts)?;

        Ok(repo)
    }

    pub fn get_active_snapshot(&self, repo: &ProjectRepository) -> anyhow::Result<Snapshot> {
        if self.config.snapshot.is_some() {
            Snapshot::from_snapshot_id(&repo.repo, self.config.snapshot.unwrap())
        } else {
            repo.get_latest_snapshot_for_branch(self, &self.config.branch)
        }
    }

    pub fn get_all_snapshots(
        &self,
        repo: &ProjectRepository,
    ) -> anyhow::Result<(Vec<Snapshot>, Vec<anyhow::Error>)> {
        repo.get_all_snapshots_for_project(self)
    }

    fn local_uploads_to_delete(
        &self,
        destination: &PathBuf,
        from_remote: HashSet<PathBuf>,
    ) -> anyhow::Result<HashSet<PathBuf>> {
        let local: HashSet<PathBuf> = glob(&format!("{}/(**/*)", destination.to_string_lossy()))
            .expect("Failed to read glob pattern")
            .flatten()
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(&local - &from_remote)
    }

    pub fn restore_from_snapshot(
        &self,
        repo: &ProjectRepository,
        snapshot: &Snapshot,
    ) -> anyhow::Result<()> {
        let destination = fs::canonicalize(&self.path)?.join(&self.config.uploads_path);

        /*
         * Disallow absolute urls - we don't want to be deleting or writing outside of our project.
         */
        if !self.config.uploads_path.is_relative() {
            return Err(anyhow::anyhow!("Project uploads path must be relative"));
        }

        /*
         * Disallow uploads traversal outside of the project
         */
        if !fs::canonicalize(&destination)?.starts_with(fs::canonicalize(&self.path)?) {
            return Err(anyhow::anyhow!(
                "Project uploads path be a child of the project itself"
            ));
        }

        /*
         * Don't allow restoring to the root of the project - this will wipe out our sprout.yaml (and everything else!)
         */
        if destination == fs::canonicalize(&self.path)? {
            return Err(anyhow::anyhow!("Project uploads path must not evaluate to the same directory as the project itself"));
        }

        let rustic_repo = repo.repo.clone().open()?.to_indexed()?;
        let uploads_node = repo.get_uploads_node(snapshot)?;
        let db_node = repo.get_db_node(snapshot)?;

        // use list of the snapshot contents using no additional filtering
        let streamer_opts = LsOptions::default();
        let ls = rustic_repo.ls(&uploads_node, &streamer_opts)?;

        let from_remote: HashSet<PathBuf> = ls
            .clone()
            .take_while(|x| x.is_ok())
            .map(|x| destination.join(&x.unwrap().0))
            .collect();

        let to_remove = self.local_uploads_to_delete(&destination, from_remote)?;

        for path in to_remove {
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else if path.is_file() {
                fs::remove_file(path)?;
            }
        }

        // restore to this destination dir
        let create = true; // create destination dir, if it doesn't exist
        let dest = LocalDestination::new(
            &destination.to_string_lossy(),
            create,
            !uploads_node.is_dir(),
        )?;

        let opts = RestoreOptions::default();
        let dry_run = false;
        // create restore infos. Note: this also already creates needed dirs in the destination
        let restore_infos = rustic_repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

        rustic_repo.restore(restore_infos, &opts, ls, &dest)?;

        let dir = tempdir()?;
        // use list of the snapshot contents using no additional filtering
        let streamer_opts = LsOptions::default();
        let ls = rustic_repo.ls(&db_node, &streamer_opts)?;

        let destination = dir.path(); // restore to this destination dir
        let create = true; // create destination dir, if it doesn't exist
        let dest =
            LocalDestination::new(&destination.to_string_lossy(), create, !db_node.is_dir())?;

        let opts = RestoreOptions::default();
        let dry_run = false;
        // create restore infos. Note: this also already creates needed dirs in the destination
        let restore_infos = rustic_repo.prepare_restore(&opts, ls.clone(), &dest, dry_run)?;

        rustic_repo.restore(restore_infos, &opts, ls, &dest)?;

        self.import_database(destination.join("database.sql"))?;

        Ok(())
    }

    #[allow(clippy::format_in_format_args)]
    pub fn print_header(&self) {
        eprintln!(
            "{:^26} {}",
            "Name:".bold().cyan().dimmed(),
            self.config.name.dimmed().italic()
        );
        eprintln!(
            "{:^26} {}",
            "Branch:".bold().cyan().dimmed(),
            self.config.branch.dimmed().italic()
        );
        eprintln!(
            "{:^26} {}",
            "Snapshot:".bold().cyan().dimmed(),
            match &self.config.snapshot {
                Some(id) => id.to_string(),
                None => "Unknown".to_string(),
            }
            .dimmed()
            .italic()
        );
        eprintln!(
            "{:^26} {}",
            "Uploads Path:".bold().cyan().dimmed(),
            self.config.uploads_path.to_string_lossy().dimmed().italic()
        );
        eprintln!(
            "{:^26} {}",
            "Remote Repository:".bold().cyan().dimmed(),
            format!(
                "{} {}",
                self.config.repo.dimmed().italic(),
                match RepositoryDefinition::get(&self.engine, &self.config.repo) {
                    Err(_) => "UNKNOWN".to_string().red(),
                    Ok((_, definition)) => match definition.repo.repository {
                        None => "INVALID".to_string().red(),
                        Some(repo_path) => format!("({})", repo_path).dimmed().italic(),
                    },
                }
            )
        );
        eprintln!();
    }
}
