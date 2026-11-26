//! Named checkpoints for rollback (S3).

#[allow(unused_imports)]
use crate::prelude::*;

use crate::error::{OmegaError, Result};
use crate::persist::KernelSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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
        use std::io::Write;
        crate::private_fs::open_options()
            .write(true)
            .create_new(true)
            .open(self.meta_path(meta.id))
            .and_then(|mut file| file.write_all(json.as_bytes()))
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
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
            let data = fs::read_to_string(&path).map_err(|e| OmegaError::Persist(e.to_string()))?;
            let meta = serde_json::from_str::<CheckpointMeta>(&data).map_err(|error| {
                OmegaError::Persist(format!(
                    "malformed checkpoint metadata {}: {error}",
                    path.display()
                ))
            })?;
            out.push(meta);
        }
        out.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(out)
    }

    pub fn load_snapshot(&self, id: Uuid) -> Result<KernelSnapshot> {
        KernelSnapshot::load_from(&self.snap_path(id))
    }
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
    fn malformed_metadata_is_an_error() {
        let dir = temp_dir().join(format!("omega-ckpt-corrupt-{}", crate::new_uuid()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("broken.meta.json"), b"{not-json}").unwrap();
        let store = CheckpointStore {
            dir: dir.clone(),
            max_keep: 32,
        };
        let error = store.list().unwrap_err().to_string();
        assert!(error.contains("malformed checkpoint metadata"), "{error}");
        let _ = fs::remove_dir_all(dir);
    }
}
