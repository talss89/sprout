use rustic_core::{repofile::SnapshotFile, Id};
use serde::Serialize;

use crate::repo::RusticRepo;

#[derive(Debug, Serialize, Clone)]
pub struct Snapshot {
    pub id: Id,
    pub snapshot: SnapshotFile,
}

pub struct SnapshotStats {
    pub new: u64,
    pub changed: u64,
    pub unmodified: u64,
    pub data_added: u64,
}

impl Snapshot {
    pub fn from_snapshot_id(repo: &RusticRepo<()>, snapshot_id: Id) -> anyhow::Result<Self> {
        let repo = repo.clone().open()?.to_indexed()?;

        let snapshot = repo.get_snapshot_from_str(&snapshot_id.to_hex(), |snap| {
            if snap.tags.contains("sprt_obj:bundle") && snap.id == snapshot_id {
                return true;
            }

            false
        })?;

        Ok(Self {
            id: snapshot.id,
            snapshot,
        })
    }

    pub fn from_snapshot(snapshot: &SnapshotFile) -> anyhow::Result<Self> {
        Ok(Self {
            id: snapshot.id,
            snapshot: snapshot.clone(),
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
                .ok_or(anyhow::anyhow!("Could not find tag {}", key))?
                .to_string());
        } else {
            Err(anyhow::anyhow!("Could not find tag {}", key))
        }
    }

    pub fn pack_stats(
        db_snapshot: &SnapshotFile,
        uploads_snapshot: &SnapshotFile,
    ) -> anyhow::Result<String> {
        let new = db_snapshot.summary.as_ref().unwrap().files_new
            + uploads_snapshot.summary.as_ref().unwrap().files_new;

        let changed = db_snapshot.summary.as_ref().unwrap().files_changed
            + uploads_snapshot.summary.as_ref().unwrap().files_changed;

        let unmodified = db_snapshot.summary.as_ref().unwrap().files_unmodified
            + uploads_snapshot.summary.as_ref().unwrap().files_unmodified;

        let data_added = db_snapshot.summary.as_ref().unwrap().data_added
            + uploads_snapshot.summary.as_ref().unwrap().data_added;

        Ok(format!("{}/{}/{}/{}", new, changed, unmodified, data_added))
    }

    pub fn get_stats(&self) -> anyhow::Result<SnapshotStats> {
        let stats = Self::get_sprout_tag(&self.snapshot, "sprt_stats")?;
        let parts: Vec<&str> = stats.split('/').collect();

        Ok(SnapshotStats {
            new: parts.get(0).unwrap_or(&&"0").parse()?,
            changed: parts.get(1).unwrap_or(&&"0").parse()?,
            unmodified: parts.get(2).unwrap_or(&&"0").parse()?,
            data_added: parts.get(3).unwrap_or(&&"0").parse()?,
        })
    }

    pub fn get_branch(&self) -> anyhow::Result<String> {
        Self::get_sprout_tag(&self.snapshot, "sprt_branch")
    }

    pub fn get_project_identity_hash(&self) -> anyhow::Result<String> {
        Self::get_sprout_tag(&self.snapshot, "sprt_uniq")
    }

    pub fn get_project_name(&self) -> String {
        self.snapshot.hostname.clone()
    }

    pub fn get_total_files(&self) -> u64 {
        if self.snapshot.summary.is_none() {
            return 0u64;
        }

        self.snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_files_processed
    }

    pub fn get_total_bytes(&self) -> u64 {
        if self.snapshot.summary.is_none() {
            return 0u64;
        }

        self.snapshot
            .summary
            .as_ref()
            .unwrap()
            .total_bytes_processed
    }
}
