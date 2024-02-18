use super::ProjectFactProvider;
use anyhow::Result;
use sha2::{Digest, Sha224};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone)]
pub struct WordPress {
    pub path: PathBuf,
}

impl WordPress {
    fn get_content_dir(&self) -> Result<String> {
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
}

impl ProjectFactProvider for WordPress {
    fn is_wordpress_installed(&self) -> Result<bool> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("core")
            .arg("is-installed")
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;

        let output = child.wait()?;

        Ok(output.success())
    }

    fn get_home_url(&self) -> Result<String> {
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

    fn get_uploads_dir(&self) -> Result<String> {
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

        if upload_path.is_empty() {
            return Ok(format!("{}/uploads", self.get_content_dir()?));
        }

        Ok(upload_path)
    }

    fn generate_unique_hash(&self) -> Result<Option<String>> {
        let mut cmd = Command::new("git");

        cmd.current_dir(&self.path)
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

    fn dump_database(&self, output_path: &Path, home_url: &str) -> Result<()> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg(home_url)
            .arg("__SPROUT__HOME__")
            .arg(format!("--export={}", output_path.to_string_lossy()))
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }
    fn import_database(&self, import_path: &Path) -> Result<()> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("db")
            .arg("import")
            .arg(import_path.as_os_str())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }

    fn postprocess_database(&self, home_url: &str) -> Result<()> {
        let mut cmd = Command::new("wp");

        cmd.current_dir(&self.path)
            .arg("search-replace")
            .arg("__SPROUT__HOME__")
            .arg(home_url)
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .stdout(Stdio::null());

        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }
}
