use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use dialoguer::Input;

use log::info;
use rustic_core::{Id, Progress, ProgressBars, Repository, RepositoryOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha224};

use crate::{
    repo::{Repositories, SproutProgressBar},
    theme::CliTheme,
};

use colored::*;

#[derive(Debug, Serialize)]
pub struct Project {
    pub path: PathBuf,
    pub config: ProjectConfig,
    pub unique_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub branch: String,
    pub snapshot: Option<String>,
    pub uploads_path: PathBuf,
    pub repo: String,
}

impl Project {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let config = Self::load_project_config(&path.join("./sprout.yaml"))?;

        Ok(Self {
            unique_hash: Project::generate_unique_hash(&path)?,
            path,
            config,
        })
    }

    pub fn initialise(path: PathBuf) -> anyhow::Result<Self> {
        if path.join("./sprout.yaml").exists() {
            return Err(anyhow::anyhow!("A sprout.yaml already exists!"));
        }

        let path = fs::canonicalize(path).unwrap();

        let config = ProjectConfig {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            branch: "main".to_string(),
            snapshot: None,
            uploads_path: PathBuf::from("./wp-content/uploads"),
            repo: "".to_string(), // TODO: Fetch last repo?
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

    pub fn get_home_url(&self) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("option")
            .arg("get")
            .arg("home")
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

    pub fn get_content_dir(&self) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
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

    pub fn get_uploads_dir(&self) -> anyhow::Result<String> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
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

        if upload_path == "" {
            return Ok(format!("{}/uploads", self.get_content_dir()?));
        }

        return Ok(upload_path);
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
            .split("\n")
            .last()
            .unwrap()
            .to_string();

        // let mut cmd = Command::new("git");

        // cmd.current_dir(path)
        //     .arg("config")
        //     .arg("get")
        //     .arg("remote.origin.url")
        //     .stderr(Stdio::null())
        //     .stdin(Stdio::null())
        //     .stdout(Stdio::piped());

        // let mut child = cmd.spawn()?;

        // let output = child.wait_with_output()?;
        // let mut origin_url = String::from_utf8_lossy(&output.stdout).into_owned();

        // if !output.status.success() {
        //     origin_url = "_none_".to_string();
        // }

        let hash = Sha224::digest(format!("{}", first_sha));

        Ok(Some(format!("{:x}", hash)))
    }

    pub fn dump_database(&self, path: &PathBuf) -> anyhow::Result<()> {
        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner(format!("Exporting database..."));

        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg(self.get_home_url()?)
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
        let home_url = self.get_home_url()?;

        let mut cmd = Command::new("wp");

        let progress = SproutProgressBar {};
        let spinner = progress.progress_spinner(format!("Importing database..."));

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
            .set_message(format!("Setting home URL to {}", &home_url));

        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg("__SPROUT__HOME__")
            .arg(&home_url)
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        spinner.finish();

        info!("Database installed and URL set to {}", &home_url);

        Ok(())
    }

    pub fn update_snapshot_id(&mut self, id: Id, branch: String) -> anyhow::Result<()> {
        self.config.snapshot = Some(id.to_string());
        self.config.branch = branch;

        fs::write(
            self.path.join("./sprout.yaml"),
            serde_yaml::to_string(&self.config)?,
        )?;

        Ok(())
    }

    pub fn open_repo(
        &self,
        access_key: String,
    ) -> anyhow::Result<Repository<SproutProgressBar, ()>> {
        let repo_opts = RepositoryOptions::default().password(access_key);
        let (_, definition) = Repositories::get(self.config.repo.as_str())?;
        let repo = crate::repo::open_repo(&definition.repo, repo_opts)?;

        Ok(repo)
    }

    pub fn snapshot(&self, repo: &Repository<SproutProgressBar, ()>) -> anyhow::Result<Id> {
        crate::repo::snapshot(repo.clone(), self)
    }

    pub fn get_latest_unique_hash(
        &self,
        repo: &Repository<SproutProgressBar, ()>,
    ) -> anyhow::Result<Option<String>> {
        let node = repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == self.config.name
                    && snap.tags.contains("sprt_obj:database")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.config.branch))
                {
                    return true;
                }

                false
            });

        match node {
            Err(_) => Ok(None),

            Ok(file) => Ok(file
                .tags
                .iter()
                .filter(|e| e.starts_with("sprt_uniq:"))
                .map(|e| e.replace("sprt_uniq:", ""))
                .collect::<Vec<String>>()
                .first()
                .cloned()),
        }
    }

    pub fn get_active_snapshot_id(
        &self,
        repo: &Repository<SproutProgressBar, ()>,
    ) -> anyhow::Result<Id> {
        let node = repo
            .clone()
            .open()?
            .to_indexed_ids()?
            .get_snapshot_from_str("latest", |snap| {
                if snap.hostname == self.config.name
                    && snap.tags.contains("sprt_obj:database")
                    && snap
                        .tags
                        .contains(&format!("sprt_branch:{}", self.config.branch))
                {
                    return true;
                }

                false
            })?;

        Ok(node.id)
    }

    pub fn print_header(&self) -> () {
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
            &self
                .config
                .snapshot
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
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
                match Repositories::get(&self.config.repo) {
                    Err(_) => "UNKNOWN".to_string().red(),
                    Ok((_, definition)) => match definition.repo.repository {
                        None => "INVALID".to_string().red(),
                        Some(repo_path) => format!("({})", repo_path).dimmed().italic(),
                    },
                }
            )
        );
        eprintln!("");
    }
}
