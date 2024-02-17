use rustic_core::{repofile::SnapshotFile, Id};
use serde::Serialize;

use crate::repo::RusticRepo;

#[derive(Debug, Serialize, Clone)]
pub struct Snapshot {
    pub id: Id,
    pub uploads_snapshot: SnapshotFile,
    pub db_snapshot: SnapshotFile,
}

impl Snapshot {
    pub fn from_db_snapshot_id(repo: &RusticRepo<()>, db_snapshot_id: Id) -> anyhow::Result<Self> {
        let repo = repo.clone().open()?.to_indexed()?;

        let db_snapshot = repo.get_snapshot_from_str(&db_snapshot_id.to_hex(), |snap| {
            if snap.tags.contains("sprt_obj:database") && snap.id == db_snapshot_id {
                return true;
            }

            false
        })?;

        let uploads_snapshot = repo.get_snapshot_from_str("latest", |snap| {
            if snap.tags.contains("sprt_obj:uploads")
                && snap
                    .tags
                    .contains(&format!("sprt_db:{}", db_snapshot_id.to_hex().as_str()))
            {
                return true;
            }

            false
        })?;

        Ok(Self {
            id: db_snapshot.id,
            db_snapshot,
            uploads_snapshot,
        })
    }

    pub fn from_db_snapshot(
        repo: &RusticRepo<()>,
        db_snapshot: &SnapshotFile,
    ) -> anyhow::Result<Self> {
        let repo = repo.clone().open()?.to_indexed()?;

        let uploads_snapshot = repo.get_snapshot_from_str("latest", |snap| {
            if snap.tags.contains("sprt_obj:uploads")
                && snap
                    .tags
                    .contains(&format!("sprt_db:{}", db_snapshot.id.to_hex().as_str()))
            {
                return true;
            }

            false
        })?;

        Ok(Self {
            id: db_snapshot.id,
            db_snapshot: db_snapshot.clone(),
            uploads_snapshot,
        })
    }

    pub fn get_sprout_tag(snapshot: &SnapshotFile, key: &str) -> anyhow::Result<String> {
        let prefix = format!("{}:", key);
        let tags = snapshot.tags.iter().filter(|t| t.starts_with(&prefix));

        if tags.clone().count() > 0 {
            return Ok(tags
                .last()
                .unwrap()
                .strip_prefix(&prefix)
                .unwrap()
                .to_string());
        } else {
            Err(anyhow::anyhow!("Could not find tag {}", key))
        }
    }

    pub fn get_branch(&self) -> anyhow::Result<String> {
        Self::get_sprout_tag(&self.db_snapshot, "sprt_branch")
    }

    pub fn get_project_name(&self) -> String {
        self.db_snapshot.hostname.clone()
    }

    pub fn get_total_files(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_files_processed
            + self
                .uploads_snapshot
                .summary
                .as_ref()
                .unwrap()
                .total_files_processed
    }

    pub fn get_total_bytes(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_bytes_processed
            + self
                .uploads_snapshot
                .summary
                .as_ref()
                .unwrap()
                .total_bytes_processed
    }

    pub fn get_data_added(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot.summary.as_ref().unwrap().data_added
            + self.uploads_snapshot.summary.as_ref().unwrap().data_added
    }

    pub fn get_files_new(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot.summary.as_ref().unwrap().files_new
            + self.uploads_snapshot.summary.as_ref().unwrap().files_new
    }

    pub fn get_files_changed(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot.summary.as_ref().unwrap().files_changed
            + self
                .uploads_snapshot
                .summary
                .as_ref()
                .unwrap()
                .files_changed
    }

    pub fn get_files_unmodified(&self) -> u64 {
        if self.db_snapshot.summary.is_none() || self.uploads_snapshot.summary.is_none() {
            return 0u64;
        }

        self.db_snapshot.summary.as_ref().unwrap().files_unmodified
            + self
                .uploads_snapshot
                .summary
                .as_ref()
                .unwrap()
                .files_unmodified
    }
}
