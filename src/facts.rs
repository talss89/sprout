use anyhow::Result;
use core::fmt::Debug;
use dyn_clone::DynClone;
use std::path::Path;

pub mod wordpress;

pub trait ProjectFactProvider: DynClone + Send + Sync {
    fn is_wordpress_installed(&self) -> Result<bool>;
    fn get_home_url(&self) -> Result<String>;
    fn get_uploads_dir(&self) -> Result<String>;
    fn generate_unique_hash(&self) -> Result<Option<String>>;
    fn dump_database(&self, output_path: &Path, home_url: &str) -> Result<()>;
    fn import_database(&self, import_path: &Path) -> Result<()>;
    fn postprocess_database(&self, home_url: &str) -> Result<()>;
}

dyn_clone::clone_trait_object!(ProjectFactProvider);

impl Debug for dyn ProjectFactProvider {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ProjectFact()")
    }
}
