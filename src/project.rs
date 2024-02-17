use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use dialoguer::Input;

use log::{info, warn};
use rustic_core::{
    Id, LocalDestination, LsOptions, Progress, ProgressBars, RepositoryOptions, RestoreOptions,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha224};
use tempfile::tempdir;

use crate::{
    engine::get_sprout_config,
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
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let config = Self::load_project_config(&path.join("./sprout.yaml"))?;

        Ok(Self {
            unique_hash: Project::generate_unique_hash(&path)?,
            path,
            home_url: format!("https://{}.test", &config.name),
            config,
        })
    }

    pub fn initialise(path: PathBuf) -> anyhow::Result<Self> {
        if path.join("./sprout.yaml").exists() {
            return Err(anyhow::anyhow!("A sprout.yaml already exists!"));
        }

        let path = fs::canonicalize(path).unwrap();

        let mut uploads_path = PathBuf::from("./wp-content/uploads");
        let sprout_config = get_sprout_config()?;

        if let Ok(installed) = Self::is_wordpress_installed(&path) {
            if installed {
                if let Ok(detected_uploads_path) = Self::get_uploads_dir(&path) {
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

        fs::write(path.join("./sprout.yaml"), serde_yaml::to_string(&config)?)?;

        Project::new(path)
    }

    pub fn load_project_config(path: &PathBuf) -> anyhow::Result<ProjectConfig> {
        Ok(serde_yaml::from_slice::<ProjectConfig>(&fs::read(path)?)?)
    }

    pub fn is_wordpress_installed(path: &PathBuf) -> anyhow::Result<bool> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(path)
            .arg("core")
            .arg("is-installed")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;

        let output = child.wait()?;

        Ok(output.success())
    }

    pub fn determine_home_url(&mut self) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Loading WordPress project with WP-CLI...");

        let home_url = match Self::get_home_url(&self.path) {
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

    pub fn get_home_url(path: &PathBuf) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(path)
            .arg("option")
            .arg("get")
            .arg("home")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());

        let child = cmd.spawn()?;

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Could not determine WordPress home URL via WP-CLI"
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .to_string()
            .trim()
            .to_string())
    }

    pub fn get_content_dir(path: &PathBuf) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(path)
            .arg("config")
            .arg("get")
            .arg("WP_CONTENT_DIR")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());

        let child = cmd.spawn()?;

        let output = child.wait_with_output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .to_string()
            .trim()
            .to_string())
    }

    pub fn get_uploads_dir(path: &PathBuf) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(path)
            .arg("option")
            .arg("get")
            .arg("upload_path")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());

        let child = cmd.spawn()?;

        let output = child.wait_with_output()?;

        let upload_path = String::from_utf8_lossy(&output.stdout)
            .to_string()
            .trim()
            .to_string();

        if upload_path.is_empty() {
            return Ok(format!("{}/uploads", Self::get_content_dir(path)?));
        }

        Ok(upload_path)
    }

    pub fn generate_unique_hash(path: &PathBuf) -> anyhow::Result<Option<String>> {
        let mut cmd = Command::new("git");

        cmd.current_dir(path)
            .arg("rev-list")
            .arg("--parents")
            .arg("HEAD")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());

        let child = cmd.spawn()?;

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let output = String::from_utf8_lossy(&output.stdout);

        let first_sha = output
            .to_string()
            .trim()
            .split('\n')
            .last()
            .unwrap()
            .to_string();

        let hash = Sha224::digest(first_sha);

        Ok(Some(format!("{:x}", hash)))
    }

    pub fn dump_database(&self, path: &Path) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Exporting database...");

        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg(&self.home_url)
            .arg("__SPROUT__HOME__")
            .arg(format!("--export={}", path.to_string_lossy()))
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        spinner.finish();

        Ok(())
    }

    pub fn import_database(&self, path: PathBuf) -> anyhow::Result<()> {
        let mut cmd = Command::new("wp");

        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner("Importing database...");

        cmd.current_dir(&self.path)
            .arg("db")
            .arg("import")
            .arg(path.as_os_str())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        spinner
            .bar
            .set_message(format!("Setting home URL to {}", &self.home_url));

        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg("__SPROUT__HOME__")
            .arg(&self.home_url)
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        spinner.finish();

        info!("Database installed and URL set to {}", &self.home_url);

        Ok(())
    }

    pub fn update_snapshot_id(&mut self, id: Id, branch: String) -> anyhow::Result<()> {
        self.config.snapshot = Some(id);
        self.config.branch = branch;

        fs::write(
            self.path.join("./sprout.yaml"),
            serde_yaml::to_string(&self.config)?,
        )?;

        Ok(())
    }

    pub fn open_repo(&self, access_key: &str) -> anyhow::Result<ProjectRepository> {
        let repo_opts = RepositoryOptions::default().password(access_key);
        let (_, definition) = RepositoryDefinition::get(self.config.repo.as_str())?;
        let repo = ProjectRepository::new(self, definition.repo, repo_opts)?;

        Ok(repo)
    }

    pub fn get_active_snapshot(&self, repo: &ProjectRepository) -> anyhow::Result<Snapshot> {
        repo.get_latest_snapshot_for_branch(self, &self.config.branch)
    }

    pub fn get_all_snapshots(
        &self,
        repo: &ProjectRepository,
    ) -> anyhow::Result<(Vec<Snapshot>, Vec<anyhow::Error>)> {
        repo.get_all_snapshots_for_project(self)
    }

    pub fn restore_from_snapshot(
        &self,
        repo: &ProjectRepository,
        snapshot: &Snapshot,
    ) -> anyhow::Result<()> {
        let rustic_repo = repo.repo.clone().open()?.to_indexed()?;
        let uploads_node = repo.get_uploads_node(snapshot)?;
        let db_node = repo.get_db_node(snapshot)?;

        // use list of the snapshot contents using no additional filtering
        let streamer_opts = LsOptions::default();
        let ls = rustic_repo.ls(&uploads_node, &streamer_opts)?;

        let destination = fs::canonicalize(&self.config.uploads_path)?; // restore to this destination dir
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
                match RepositoryDefinition::get(&self.config.repo) {
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
