//! Named checkpoints for rollback (S3).

#[allow(unused_imports)]
use crate::prelude::*;

use crate::error::{OmegaError, Result};
use crate::persist::KernelSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const METADATA_TEMP_PREFIX: &str = ".checkpoint-metadata-";
const METADATA_TEMP_SUFFIX: &str = ".tmp";
const STALE_METADATA_TEMP_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub id: Uuid,
    pub label: String,
    pub step: u64,
    pub created_at: DateTime<Utc>,
    pub avg_err: f64,
    pub damping: f64,
}

pub struct CheckpointStore {
    pub dir: PathBuf,
    pub max_keep: usize,
}

impl CheckpointStore {
    pub fn beside_state(state_path: &Path) -> Self {
        let dir = state_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("checkpoints");
        Self { dir, max_keep: 32 }
    }

    fn meta_path(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.meta.json"))
    }

    fn snap_path(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.state.json"))
    }

    pub fn save(
        &self,
        label: &str,
        step: u64,
        avg_err: f64,
        damping: f64,
        snapshot: &KernelSnapshot,
    ) -> Result<CheckpointMeta> {
        self.save_with_privacy(label, step, avg_err, damping, snapshot, false)
    }

    /// Save a checkpoint, optionally redacting known user-provided content.
    pub fn save_with_privacy(
        &self,
        label: &str,
        step: u64,
        avg_err: f64,
        damping: f64,
        snapshot: &KernelSnapshot,
        redact_user_content: bool,
    ) -> Result<CheckpointMeta> {
        crate::private_fs::create_dir_all(&self.dir)
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
        self.cleanup_stale_metadata_temps();
        let meta = CheckpointMeta {
            id: crate::new_uuid(),
            label: if redact_user_content {
                "[REDACTED]".into()
            } else {
                label.into()
            },
            step,
            created_at: crate::now_utc(),
            avg_err,
            damping,
        };
        let mut persisted_snapshot = snapshot.clone();
        if redact_user_content {
            persisted_snapshot.redact_user_content();
        }
        persisted_snapshot.save_to(&self.snap_path(meta.id))?;
        let json =
            serde_json::to_string_pretty(&meta).map_err(|e| OmegaError::Persist(e.to_string()))?;
        write_metadata_atomically(&self.meta_path(meta.id), json.as_bytes())?;
        self.prune()?;
        Ok(meta)
    }

    fn prune(&self) -> Result<()> {
        let mut list = self.list()?;
        if list.len() <= self.max_keep {
            return Ok(());
        }
        list.sort_by_key(|a| a.created_at);
        let excess = list.len().saturating_sub(self.max_keep);
        for meta in list.into_iter().take(excess) {
            let _ = fs::remove_file(self.meta_path(meta.id));
            let _ = fs::remove_file(self.snap_path(meta.id));
        }
        Ok(())
    }

    fn cleanup_stale_metadata_temps(&self) {
        self.cleanup_metadata_temps_older_than(STALE_METADATA_TEMP_AGE);
    }

    fn cleanup_metadata_temps_older_than(&self, minimum_age: Duration) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !name.starts_with(METADATA_TEMP_PREFIX) || !name.ends_with(METADATA_TEMP_SUFFIX) {
                continue;
            }
            let stale = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age >= minimum_age);
            if stale {
                if let Err(error) = fs::remove_file(entry.path()) {
                    tracing::warn!(
                        metadata_temp = %entry.path().display(),
                        error = %error,
                        "failed to remove stale checkpoint metadata temp file"
                    );
                }
            }
        }
    }

    pub fn list(&self) -> Result<Vec<CheckpointMeta>> {
        if !self.dir.exists() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(|e| OmegaError::Persist(e.to_string()))? {
            let entry = entry.map_err(|e| OmegaError::Persist(e.to_string()))?;
            let path = entry.path();
            if !path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".meta.json"))
                .unwrap_or(false)
            {
                continue;
            }
            let data = match fs::read_to_string(&path) {
                Ok(data) => data,
                Err(error) => {
                    tracing::warn!(
                        metadata_path = %path.display(),
                        error = %error,
                        "skipping unreadable checkpoint metadata"
                    );
                    continue;
                }
            };
            let meta = match serde_json::from_str::<CheckpointMeta>(&data) {
                Ok(meta) => meta,
                Err(error) => {
                    tracing::warn!(
                        metadata_path = %path.display(),
                        error = %error,
                        "skipping malformed checkpoint metadata"
                    );
                    continue;
                }
            };
            out.push(meta);
        }
        out.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(out)
    }

    pub fn load_snapshot(&self, id: Uuid) -> Result<KernelSnapshot> {
        KernelSnapshot::load_from(&self.snap_path(id))
    }
}

fn write_metadata_atomically(path: &Path, json: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp_path = parent.join(format!(
        "{METADATA_TEMP_PREFIX}{}{METADATA_TEMP_SUFFIX}",
        crate::new_uuid()
    ));
    let result = (|| {
        use std::io::Write;
        let mut file = crate::private_fs::open_options()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                OmegaError::Persist(format!("checkpoint metadata temp create: {error}"))
            })?;
        file.write_all(json)
            .and_then(|()| file.sync_all())
            .map_err(|error| {
                OmegaError::Persist(format!("checkpoint metadata temp write: {error}"))
            })?;
        fs::hard_link(&temp_path, path).map_err(|error| {
            OmegaError::Persist(format!("checkpoint metadata publish: {error}"))
        })?;
        Ok(())
    })();
    let _ = fs::remove_file(temp_path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuleEffect;
    use hashbrown::HashMap;
    use std::env::temp_dir;

    #[test]
    fn save_list_load() {
        let dir = temp_dir().join(format!("omega-ckpt-{}", crate::new_uuid()));
        let store = CheckpointStore {
            dir: dir.clone(),
            max_keep: 32,
        };
        let snap = KernelSnapshot {
            version: 1,
            total_steps: 3,
            biases: HashMap::new(),
            constraints: HashMap::new(),
            cas_total_steps: 3,
            cas_avg_prediction_error: 0.4,
            cas_adaptation_score: 0.6,
            cognitive_effect: RuleEffect::default(),
            cases: vec![],
            case_capacity: 256,
            strategies: HashMap::new(),
            active_modules: HashMap::new(),
            patch_history: vec![],
            ..KernelSnapshot::default()
        };
        let meta = store.save("before-patch", 3, 0.4, 1.0, &snap).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        assert_eq!(store.load_snapshot(meta.id).unwrap().total_steps, 3);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(store.snap_path(meta.id))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(store.meta_path(meta.id))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_metadata_does_not_block_listing_or_pruning() {
        let dir = temp_dir().join(format!("omega-ckpt-corrupt-{}", crate::new_uuid()));
        let store = CheckpointStore {
            dir: dir.clone(),
            max_keep: 1,
        };
        let snapshot = KernelSnapshot::default();
        let first = store.save("first", 1, 0.0, 1.0, &snapshot).unwrap();
        fs::write(dir.join("broken.meta.json"), b"{not-json}").unwrap();
        let second = store.save("second", 2, 0.0, 1.0, &snapshot).unwrap();

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].id == first.id || listed[0].id == second.id);
        store.prune().unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        assert!(dir.join("broken.meta.json").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_metadata_temp_files_are_removed_without_touching_other_files() {
        let dir = temp_dir().join(format!("omega-ckpt-temp-{}", crate::new_uuid()));
        fs::create_dir_all(&dir).unwrap();
        let store = CheckpointStore {
            dir: dir.clone(),
            max_keep: 32,
        };
        let stale_temp = dir.join(format!(
            "{METADATA_TEMP_PREFIX}orphan{METADATA_TEMP_SUFFIX}"
        ));
        let unrelated = dir.join("unrelated.tmp");
        fs::write(&stale_temp, b"partial metadata").unwrap();
        fs::write(&unrelated, b"leave me alone").unwrap();

        store.cleanup_metadata_temps_older_than(Duration::ZERO);

        assert!(!stale_temp.exists());
        assert!(unrelated.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
