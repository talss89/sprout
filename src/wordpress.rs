use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub struct WordPress {
    pub path: PathBuf,
}

impl WordPress {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        if !WordPress::is_wordpress(&path)? {
            return Err(anyhow::anyhow!("This is not a WordPress project"));
        }

        Ok(Self { path })
    }

    pub fn is_wordpress(path: &PathBuf) -> anyhow::Result<bool> {
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

        let mut child = cmd.spawn()?;

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

        let mut child = cmd.spawn()?;

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

        let mut child = cmd.spawn()?;

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

    pub fn get_project_identifier(&self) -> anyhow::Result<String> {
        let mut cmd = Command::new("git");

        cmd.current_dir(&self.path)
            .arg("rev-list")
            .arg("--parents")
            .arg("HEAD")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::piped());

        let mut child = cmd.spawn()?;

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get root commit SHA"));
        }

        let output = String::from_utf8_lossy(&output.stdout);

        Ok(output
            .to_string()
            .trim()
            .split("\n")
            .last()
            .unwrap()
            .to_string())
    }

    pub fn dump_database(&self, path: &PathBuf) -> anyhow::Result<()> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg(self.get_home_url()?)
            .arg("__SPROUT__HOME__")
            .arg(format!("--export={}", path.to_string_lossy()));

        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }
}
