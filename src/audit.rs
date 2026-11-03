//! Append-only audit log (S3 hardening).

use crate::error::{OmegaError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    Step {
        step: u64,
        goal: String,
        action: String,
        avg_err: f64,
        damping: f64,
    },
    PatchAccepted {
        step: u64,
        patch_id: Uuid,
        damping: f64,
        description: String,
    },
    PatchRejected {
        step: u64,
        reason: String,
    },
    Checkpoint {
        checkpoint_id: Uuid,
        step: u64,
        label: String,
    },
    Rollback {
        checkpoint_id: Uuid,
        step_before: u64,
    },
    TaskEpisode {
        task: String,
        reward: f64,
        steps: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: DateTime<Utc>,
    pub event: AuditEvent,
}

pub struct AuditLog {
    pub path: PathBuf,
}

impl AuditLog {
    pub fn beside_state(state_path: &Path) -> Self {
        let path = state_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("audit.jsonl");
        Self { path }
    }

    pub fn append(&self, event: AuditEvent) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let rec = AuditRecord {
            ts: Utc::now(),
            event,
        };
        let line = serde_json::to_string(&rec).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| OmegaError::Persist(e.to_string()))?;
        writeln!(f, "{line}").map_err(|e| OmegaError::Persist(e.to_string()))?;
        Ok(())
    }

    pub fn tail(&self, n: usize) -> Result<Vec<AuditRecord>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let data =
            fs::read_to_string(&self.path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        let mut rows: Vec<AuditRecord> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        if rows.len() > n {
            rows = rows.split_off(rows.len() - n);
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn append_and_tail() {
        let path = temp_dir().join(format!("omega-audit-{}.jsonl", Uuid::new_v4()));
        let log = AuditLog { path: path.clone() };
        log.append(AuditEvent::Step {
            step: 1,
            goal: "g".into(),
            action: "observe".into(),
            avg_err: 0.5,
            damping: 1.0,
        })
        .unwrap();
        let rows = log.tail(10).unwrap();
        assert_eq!(rows.len(), 1);
        let _ = fs::remove_file(&path);
    }
}
