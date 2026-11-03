//! Named checkpoints for rollback (S3).

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
        fs::create_dir_all(&self.dir).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let meta = CheckpointMeta {
            id: Uuid::new_v4(),
            label: label.into(),
            step,
            created_at: Utc::now(),
            avg_err,
            damping,
        };
        snapshot.save_to(&self.snap_path(meta.id))?;
        let json =
            serde_json::to_string_pretty(&meta).map_err(|e| OmegaError::Persist(e.to_string()))?;
        fs::write(self.meta_path(meta.id), json).map_err(|e| OmegaError::Persist(e.to_string()))?;
        self.prune()?;
        Ok(meta)
    }

    fn prune(&self) -> Result<()> {
        let mut list = self.list()?;
        if list.len() <= self.max_keep {
            return Ok(());
        }
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
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
            if let Ok(m) = serde_json::from_str::<CheckpointMeta>(&data) {
                out.push(m);
            }
        }
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
    use std::collections::HashMap;
    use std::env::temp_dir;

    #[test]
    fn save_list_load() {
        let dir = temp_dir().join(format!("omega-ckpt-{}", Uuid::new_v4()));
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
        };
        let meta = store.save("before-patch", 3, 0.4, 1.0, &snap).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        assert_eq!(store.load_snapshot(meta.id).unwrap().total_steps, 3);
        let _ = fs::remove_dir_all(&dir);
    }
}
