use rustic_core::{
    repofile::{Node, SnapshotFile},
    Id,
};

use crate::repo::RusticRepo;

pub struct Snapshot {
    pub id: Id,
    pub uploads_snapshot: SnapshotFile,
    pub db_snapshot: SnapshotFile,
}

impl Snapshot {
    pub fn from_db_snapshot_id(repo: &RusticRepo<()>, db_snapshot_id: Id) -> anyhow::Result<Self> {
        let repo = repo.clone().open()?.to_indexed()?;

        let db_snapshot =
            repo.get_snapshot_from_str(&db_snapshot_id.to_hex().to_string(), |snap| {
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
}
