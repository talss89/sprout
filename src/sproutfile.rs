use std::path::PathBuf;

use rustic_backend::BackendOptions;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SproutFile {
    pub name: String,
    pub branch: String,
    pub snapshot: Option<String>,
    pub uploads_path: PathBuf,
    pub repo: BackendOptions,
}
