use anyhow::Result;
use sprout::facts::ProjectFactProvider;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct FixtureFacts {
    pub path: PathBuf,
    pub is_wordpress_installed: bool,
    pub home_url: String,
    pub hash: Option<String>,
}

impl FixtureFacts {}

impl ProjectFactProvider for FixtureFacts {
    fn is_wordpress_installed(&self) -> Result<bool> {
        Ok(self.is_wordpress_installed)
    }

    fn get_home_url(&self) -> Result<String> {
        Ok(self.home_url.to_owned())
    }

    fn get_uploads_dir(&self) -> Result<String> {
        let uploads_path = self.path.join("uploads/");
        if !uploads_path.exists() {
            fs::create_dir(&uploads_path)?;
        }

        Ok(fs::canonicalize(uploads_path)
            .unwrap()
            .to_string_lossy()
            .to_string())
    }

    fn generate_unique_hash(&self) -> Result<Option<String>> {
        Ok(self.hash.to_owned())
    }

    fn dump_database(&self, output_path: &Path, _home_url: &str) -> Result<()> {
        fs::write(output_path, "-- This is a test fixture\nSHOW DATABASES;")?;
        Ok(())
    }
    fn import_database(&self, _import_path: &Path) -> Result<()> {
        Ok(())
    }

    fn postprocess_database(&self, _home_url: &str) -> Result<()> {
        Ok(())
    }
}
